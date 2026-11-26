//! The tap on a daemon's inferd link: every chat request and its answer, as the daemon saw
//! them, kept as [`ModelExchange`] lines for a live run's trace and for turning a live failure
//! into a cassette. It is the one place the words a model was sent and said can be read back:
//! inferd's own audit never keeps content, and its replay `record` sees requests only.
//!
//! THIS WRITES PROMPTS TO DISK. Only a caller that builds a [`Tap::trace_file`] turns it on, and
//! only the live harness's mains (`action-accept`) do: they read the model-trace variable, which
//! names a file (in the harness's scratch directory; the file is created 0600 and appended to, one
//! JSON line per exchange) and pass the tap in. No daemon's `main` reads an environment variable
//! for it, so a release daemon's environment cannot make it write prompts;
//! `cargo xtask check layers` checks that no shipped crate names the variable. A daemon that is
//! not asked takes no copy of anything. A process that is not a daemon builds a [`MemoryTap`] and
//! reads the exchanges itself.

use crate::exchange::{begin, finish, route_note};
use action_core::ModelExchange;
use action_dbus::{InferObserver, InferWatch, SessionWatch};
use inference_core::{ClientFrame, InferEvent, InferRequest};
use std::io::Write;
use std::os::unix::fs::OpenOptionsExt;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, Mutex, PoisonError};
use std::time::Instant;

/// Exchanges kept in memory, for the process that runs the whole case itself.
#[derive(Debug, Clone, Default)]
pub struct MemoryTap(Arc<Mutex<Vec<ModelExchange>>>);

impl MemoryTap {
    /// Takes every exchange kept so far, oldest first, leaving none.
    pub fn take(&self) -> Vec<ModelExchange> {
        std::mem::take(&mut *self.0.lock().unwrap_or_else(PoisonError::into_inner))
    }
}

#[derive(Debug, Clone)]
enum Sink {
    Off,
    File(Arc<PathBuf>),
    Memory(MemoryTap),
}

/// Where exchanges go, and who is asking.
#[derive(Debug, Clone)]
pub struct Tap {
    sink: Sink,
    by: Arc<str>,
    count: Arc<AtomicU32>,
}

/// The name of this process for a trace: its executable's name without the harness prefix.
fn process_label() -> String {
    std::env::current_exe()
        .ok()
        .and_then(|p| p.file_stem().map(|s| s.to_string_lossy().into_owned()))
        .map_or_else(
            || "unknown".to_owned(),
            |name| {
                name.strip_prefix("accept-")
                    .map_or(name.clone(), str::to_owned)
            },
        )
}

impl Tap {
    fn with(sink: Sink, by: &str) -> Self {
        Self {
            sink,
            by: Arc::from(by),
            count: Arc::default(),
        }
    }

    /// A tap that keeps nothing.
    #[must_use]
    pub fn off() -> Self {
        Self::with(Sink::Off, "")
    }

    /// A tap that appends to `path`, as `by`.
    #[must_use]
    pub fn file(path: PathBuf, by: &str) -> Self {
        Self::with(Sink::File(Arc::new(path)), by)
    }

    /// A tap that keeps exchanges in `memory`, as `by`.
    #[must_use]
    pub fn memory(memory: MemoryTap, by: &str) -> Self {
        Self::with(Sink::Memory(memory), by)
    }

    /// A tap that appends to `path`, labelled with this process's executable name.
    #[must_use]
    pub fn trace_file(path: PathBuf) -> Self {
        Self::file(path, &process_label())
    }

    /// The watch to hand a daemon's inferd link: this tap, or none when it keeps nothing.
    #[must_use]
    pub fn watch(&self) -> InferWatch {
        if self.is_off() {
            InferWatch::off()
        } else {
            InferWatch::new(Arc::new(self.clone()))
        }
    }

    fn is_off(&self) -> bool {
        matches!(self.sink, Sink::Off)
    }

    fn keep(&self, exchange: ModelExchange) {
        match &self.sink {
            Sink::Off => {}
            Sink::Memory(memory) => memory
                .0
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .push(exchange),
            Sink::File(path) => {
                if let Err(why) = append(path, &exchange) {
                    tracing::warn!(path = %path.display(), %why, "the model trace could not be written");
                }
            }
        }
    }
}

fn append(path: &PathBuf, exchange: &ModelExchange) -> std::io::Result<()> {
    let mut line = serde_json::to_string(exchange).map_err(std::io::Error::other)?;
    line.push('\n');
    std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .mode(0o600)
        .open(path)?
        .write_all(line.as_bytes())
}

impl InferObserver for Tap {
    fn session(&self) -> Box<dyn SessionWatch> {
        Box::new(TapSession {
            tap: self.clone(),
            open: None,
        })
    }
}

/// One session's watcher: the exchange in flight, if a chat turn is open.
#[derive(Debug)]
struct TapSession {
    tap: Tap,
    open: Option<(ModelExchange, Instant)>,
}

impl SessionWatch for TapSession {
    fn sent(&mut self, frame: &ClientFrame) {
        if self.tap.is_off() {
            return;
        }
        if let ClientFrame::Request(InferRequest::Chat(request)) = frame {
            let n = self.tap.count.fetch_add(1, Ordering::Relaxed) + 1;
            self.open = Some((begin(&self.tap.by, n, request), Instant::now()));
        }
    }

    fn received(&mut self, event: &InferEvent) {
        let Some((open, began)) = self.open.as_mut() else {
            return;
        };
        if let Some(note) = route_note(event) {
            open.route.push(note);
        }
        if let InferEvent::Finished(reply) = event {
            let took = u32::try_from(began.elapsed().as_millis()).unwrap_or(u32::MAX);
            if let Some((open, _)) = self.open.take() {
                self.tap.keep(finish(open, reply, took));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use account_capability::Tier;
    use account_core::DataClass;
    use inference_core::{ChatRequest, InferReply, ReplyShape};

    fn sample() -> ChatRequest {
        ChatRequest::new(
            vec![],
            Tier::Fast,
            DataClass::Prompt,
            account_core::consent::Usage::Interactive,
        )
        .with_shape(ReplyShape::Text)
        .with_control(
            inference_core::ChatControl::new()
                .with_tool_choice(inference_core::ToolChoice::Never)
                .with_tool_calls(inference_core::ToolParallelism::One)
                .with_max_output(inference_core::Knob::Off)
                .with_reasoning(inference_core::Reasoning::Off)
                .with_sampling(inference_core::Knob::Off),
        )
    }

    #[test]
    fn a_memory_tap_hands_each_exchange_over_once_in_order() {
        let memory = MemoryTap::default();
        let tap = Tap::memory(memory.clone(), "eval");
        for n in 1..=2 {
            tap.keep(finish(
                begin("eval", n, &sample()),
                &InferReply::Cancelled,
                1,
            ));
        }
        let taken = memory.take();
        assert_eq!(taken.iter().map(|e| e.n).collect::<Vec<_>>(), [1, 2]);
        assert!(memory.take().is_empty());
        assert_eq!(taken[0].tier, "fast");
        assert_eq!(taken[0].class, "prompt");
    }
}
