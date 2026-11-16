//! Suspend on a tty seat (P1-T): the machine going to sleep reaches the session as a pause, and
//! waking as a return, so the displays are re-modeset after the sleep.
//!
//! `compositor_ipc::sleep` hears logind on the system bus; this file joins it to the session's
//! [`SleepLink`].

use crate::state::State;
use compositor_backend::BackendKind;
use compositor_backend::tty::{SleepLink, SleepSettled};
use compositor_ipc::sleep::{SleepError, SleepHandler, SleepWatch};
use std::time::Duration;

/// How long logind is held back for the compositor to let go of its devices. Well under
/// logind's own inhibitor delay (5 s).
const LET_GO_WITHIN: Duration = Duration::from_secs(2);

/// Passes logind's word to the session.
struct Linked(SleepLink);

impl SleepHandler for Linked {
    fn starting(&mut self) {
        match self.0.paused(LET_GO_WITHIN) {
            SleepSettled::Done | SleepSettled::Gone => {}
            SleepSettled::TimedOut => {
                tracing::warn!("the displays were not given back before the sleep");
            }
        }
    }

    fn resumed(&mut self) {
        self.0.activated();
    }
}

/// Hears logind on the bus at `bus` (the system bus when none) and passes a sleep and a wake to `link`. The watcher lives as long as
/// the returned handle.
///
/// ```no_run
/// use compositor_app::suspend::watch;
/// use compositor_backend::tty::sleep_channel;
///
/// let (link, _inbox) = sleep_channel(|| {});
/// let _watch = watch(link, None).expect("login1 is on the bus");
/// ```
///
/// # Errors
///
/// Returns [`SleepError`] when the watcher cannot connect to the bus or subscribe to logind.
pub fn watch(link: SleepLink, bus: Option<String>) -> Result<SleepWatch, SleepError> {
    SleepWatch::start(bus, Linked(link))
}

/// Starts hearing logind on `bus` when the backend is a seat whose session can hear it.
///
/// The watcher lives as long as the returned handle; a seat that cannot hear a suspend, or a bus
/// that cannot be reached, is logged and the run goes on without (displays then come back from a
/// sleep as the kernel left them).
pub fn attach(state: &mut State, bus: Option<String>) -> Option<SleepWatch> {
    let BackendKind::Tty(tty) = &mut state.backend else {
        return None;
    };
    let link = tty.seams.session.sleep_link()?;
    match watch(link, bus) {
        Ok(watch) => Some(watch),
        Err(error) => {
            tracing::warn!(%error, "suspend will not be heard");
            None
        }
    }
}
