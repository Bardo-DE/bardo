//! Following the desktop's colour scheme over the settings portal.
//!
//! A thread at the binary's edge that reads the scheme once and then tells the compositor each time
//! it changes, as `sleep` and `signals` do for their buses. The compositor names no bus type: the
//! one D-Bus crate is this one (A4). The scheme is told as whether it is dark (no preference is
//! light); the compositor maps it to its own scheme type.
//!
//! The source is a seam: [`SchemeSource::Portal`] reaches the session bus; a test passes
//! [`SchemeSource::Fixed`] and never does.
//!
//! ```
//! use compositor_ipc::system_scheme::{SchemeSource, SchemeWatch};
//! use std::ops::ControlFlow;
//! use std::sync::mpsc;
//!
//! let (tx, rx) = mpsc::channel();
//! let _watch = SchemeWatch::start(SchemeSource::Fixed { dark: false }, move |dark| {
//!     match tx.send(dark) {
//!         Ok(()) => ControlFlow::Continue(()),
//!         Err(_) => ControlFlow::Break(()),
//!     }
//! })
//! .expect("a thread starts");
//! assert_eq!(rx.recv(), Ok(false));
//! ```

use crate::stop_on_drop::StopOnDrop;
use base_spawn::Spawner;
use settings_portal::Watch;
use std::ops::ControlFlow;
use std::pin::Pin;
use tokio::sync::oneshot;

/// Where the desktop's scheme is read from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SchemeSource {
    /// The settings portal on the session bus. A desktop without it answers light and never
    /// changes.
    Portal,
    /// This answer, forever: a test, or a stand-in config home that never reaches a bus.
    Fixed {
        /// Whether the fixed scheme is dark.
        dark: bool,
    },
}

/// Why the watcher could not start.
#[derive(Debug, thiserror::Error)]
pub enum SchemeError {
    /// The runtime or its thread could not be started.
    #[error("the colour scheme watcher: {0}")]
    Thread(#[source] std::io::Error),
}

/// Runs the portal's task on the watcher's own runtime.
struct InRuntime(tokio::runtime::Handle);

impl Spawner for InRuntime {
    fn spawn(&self, task: Pin<Box<dyn Future<Output = ()> + Send>>) {
        drop(self.0.spawn(task));
    }
}

/// The running watcher; dropping it stops the thread.
#[derive(Debug)]
pub struct SchemeWatch {
    _guard: StopOnDrop,
}

impl SchemeWatch {
    /// Reads `source`'s colour scheme and passes whether it is dark to `tell` (from the watcher's
    /// thread), then passes each change. `tell` returning `Break` ends the watch. A desktop
    /// without the portal is light once and never a change.
    ///
    /// # Errors
    ///
    /// Returns [`SchemeError::Thread`] when the watcher's thread cannot be started.
    pub fn start(
        source: SchemeSource,
        tell: impl FnMut(bool) -> ControlFlow<()> + Send + 'static,
    ) -> Result<SchemeWatch, SchemeError> {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(SchemeError::Thread)?;
        let (stop, stopped) = oneshot::channel();
        let thread = std::thread::Builder::new()
            .name("compositor-scheme".to_owned())
            .spawn(move || runtime.block_on(follow(source, tell, stopped)))
            .map_err(SchemeError::Thread)?;
        Ok(SchemeWatch {
            _guard: StopOnDrop::new(stop, thread),
        })
    }
}

/// The first answer, then every change, until `tell` breaks, the portal goes or `stopped` fires.
/// A fixed source answers once.
async fn follow(
    source: SchemeSource,
    mut tell: impl FnMut(bool) -> ControlFlow<()>,
    mut stopped: oneshot::Receiver<()>,
) {
    if let SchemeSource::Fixed { dark } = source {
        let _ = tell(dark);
        return;
    }
    let spawner = InRuntime(tokio::runtime::Handle::current());
    let mut watch = Watch::start(&spawner).await;
    if tell(watch.current().dark).is_break() {
        return;
    }
    loop {
        tokio::select! {
            _ = &mut stopped => return,
            next = watch.changed() => match next {
                Some(prefs) => {
                    if tell(prefs.dark).is_break() {
                        return;
                    }
                }
                None => return,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::mpsc;

    fn watching(source: SchemeSource) -> (SchemeWatch, mpsc::Receiver<bool>) {
        let (tx, rx) = mpsc::channel();
        let watch = SchemeWatch::start(source, move |dark| match tx.send(dark) {
            Ok(()) => ControlFlow::Continue(()),
            Err(_) => ControlFlow::Break(()),
        })
        .expect("a thread starts");
        (watch, rx)
    }

    #[test]
    fn a_fixed_source_answers_its_scheme_and_never_touches_the_bus() {
        for dark in [false, true] {
            let (_watch, rx) = watching(SchemeSource::Fixed { dark });
            assert_eq!(rx.recv(), Ok(dark));
        }
    }

    #[test]
    fn a_fixed_source_never_changes() {
        let (watch, rx) = watching(SchemeSource::Fixed { dark: true });
        assert_eq!(rx.recv(), Ok(true));
        drop(watch);
        assert!(rx.recv().is_err(), "nothing more, and the thread is gone");
    }
}
