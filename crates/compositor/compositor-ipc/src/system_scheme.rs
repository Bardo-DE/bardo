//! Following the desktop's colour scheme over the settings portal: a thread at the binary's edge
//! that reads the scheme once and then tells the compositor each time it changes, as `sleep` and
//! `signals` do for their buses. The compositor names no bus type: the one D-Bus crate is this
//! one (A4). The scheme is `ui_style`'s own `Scheme` (no preference is light).
//!
//! The source is a seam: [`SystemPrefsSource::Portal`] reaches the session bus; a test passes
//! [`SystemPrefsSource::Fixed`] and never does.
//!
//! ```
//! use compositor_ipc::system_scheme::SchemeWatch;
//! use ui_settings::SystemPrefsSource;
//! use ui_style::appearance::theme::Scheme;
//! use std::ops::ControlFlow;
//! use std::sync::mpsc;
//!
//! let (tx, rx) = mpsc::channel();
//! let _watch = SchemeWatch::start(SystemPrefsSource::Fixed(Default::default()), move |scheme| {
//!     match tx.send(scheme) {
//!         Ok(()) => ControlFlow::Continue(()),
//!         Err(_) => ControlFlow::Break(()),
//!     }
//! })
//! .expect("a thread starts");
//! assert_eq!(rx.recv(), Ok(Scheme::Light));
//! ```

use crate::stop_on_drop::StopOnDrop;
use std::future::Future;
use std::ops::ControlFlow;
use std::pin::Pin;
use tokio::sync::oneshot;
use ui_core::spawner::Spawner;
use ui_settings::{SystemPrefsSource, SystemPrefsWatch};
use ui_style::appearance::theme::Scheme;

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
    /// Reads `source`'s colour scheme and passes it to `tell` (from the watcher's thread), then
    /// passes each change. `tell` returning `Break` ends the watch. A desktop without the portal
    /// is the default scheme once and never a change.
    pub fn start(
        source: SystemPrefsSource,
        tell: impl FnMut(Scheme) -> ControlFlow<()> + Send + 'static,
    ) -> Result<SchemeWatch, SchemeError> {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(SchemeError::Thread)?;
        let (stop, stopped) = oneshot::channel();
        let thread = std::thread::Builder::new()
            .name("casement-scheme".to_owned())
            .spawn(move || runtime.block_on(follow(source, tell, stopped)))
            .map_err(SchemeError::Thread)?;
        Ok(SchemeWatch {
            _guard: StopOnDrop::new(stop, thread),
        })
    }
}

/// The first answer, then every change, until `tell` breaks, the portal goes or `stopped` fires.
async fn follow(
    source: SystemPrefsSource,
    mut tell: impl FnMut(Scheme) -> ControlFlow<()>,
    mut stopped: oneshot::Receiver<()>,
) {
    let spawner = InRuntime(tokio::runtime::Handle::current());
    let mut watch = SystemPrefsWatch::start(&source, &spawner).await;
    if tell(watch.current().scheme).is_break() {
        return;
    }
    loop {
        tokio::select! {
            _ = &mut stopped => return,
            next = watch.changed() => match next {
                Some(prefs) => {
                    if tell(prefs.scheme).is_break() {
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
    use ui_settings::scheme_from_portal;

    /// A fixed source preferring the portal's `color-scheme` `code`.
    fn fixed(code: u32) -> SystemPrefsSource {
        let mut source = SystemPrefsSource::Fixed(Default::default());
        if let SystemPrefsSource::Fixed(prefs) = &mut source {
            prefs.scheme = scheme_from_portal(code);
        }
        source
    }

    fn watching(source: SystemPrefsSource) -> (SchemeWatch, mpsc::Receiver<Scheme>) {
        let (tx, rx) = mpsc::channel();
        let watch = SchemeWatch::start(source, move |scheme| match tx.send(scheme) {
            Ok(()) => ControlFlow::Continue(()),
            Err(_) => ControlFlow::Break(()),
        })
        .expect("a thread starts");
        (watch, rx)
    }

    #[test]
    fn a_fixed_source_answers_its_scheme_and_never_touches_the_bus() {
        for code in [0, 1, 2] {
            let (_watch, rx) = watching(fixed(code));
            assert_eq!(rx.recv(), Ok(scheme_from_portal(code)), "code {code}");
        }
    }

    #[test]
    fn a_fixed_source_never_changes() {
        let (watch, rx) = watching(fixed(1));
        assert_eq!(rx.recv(), Ok(Scheme::Dark));
        drop(watch);
        assert!(rx.recv().is_err(), "nothing more, and the thread is gone");
    }
}
