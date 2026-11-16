//! The focus mode, published and settable: the mode in effect (which follows the settled
//! settings) and a setter whose sets the daemon's focus task writes into `settings.toml`
//! (`shell_app::daemon::settings::focus`). The daemon builds one and hands it to every surface as a
//! root context beside `SettingsFeed` (`crate::hooks::use_focus_mode`).
//!
//! Setting goes through the file, never around it: the write is seen by the settings watch like
//! any other edit, the feed settles, and the mode follows. So the file stays the one truth, and a
//! hand edit and a toggle cannot disagree.

use tokio::sync::{mpsc, watch};

/// Whether a set reached the focus task.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Delivery {
    Queued,
    /// The task has stopped (the daemon is exiting); the mode is unchanged.
    Gone,
}

/// The focus mode's receiver and its setter. Clone is cheap. A watch receiver and a sender,
/// never a Dioxus signal.
#[derive(Debug, Clone)]
pub struct FocusLink {
    mode: watch::Receiver<bool>,
    set: mpsc::UnboundedSender<bool>,
}

impl FocusLink {
    /// A link from its halves (the task's, or a test's).
    #[must_use]
    pub fn new(mode: watch::Receiver<bool>, set: mpsc::UnboundedSender<bool>) -> Self {
        FocusLink { mode, set }
    }

    /// A link fixed at `mode` whose sets go nowhere, for a headless render or a test.
    #[must_use]
    pub fn fixed(mode: bool) -> FocusLink {
        let (_tx, rx) = watch::channel(mode);
        let (set, _commands) = mpsc::unbounded_channel();
        FocusLink { mode: rx, set }
    }

    /// A receiver of the mode in effect.
    #[must_use]
    pub fn mode(&self) -> watch::Receiver<bool> {
        self.mode.clone()
    }

    /// Ask for `mode`: stored in `settings.toml`, published once the file settles.
    #[must_use]
    pub fn set(&self, mode: bool) -> Delivery {
        match self.set.send(mode) {
            Ok(()) => Delivery::Queued,
            Err(_) => Delivery::Gone,
        }
    }
}

#[cfg(test)]
mod tests {
    use tokio::sync::{mpsc, watch};

    use super::{Delivery, FocusLink};

    #[test]
    fn a_set_reaches_the_task_and_a_stopped_task_is_gone() {
        let (_mode_tx, mode) = watch::channel(false);
        let (set, mut commands) = mpsc::unbounded_channel();
        let link = FocusLink::new(mode, set);
        assert_eq!(link.set(true), Delivery::Queued);
        assert_eq!(commands.try_recv().ok(), Some(true));
        drop(commands);
        assert_eq!(link.set(false), Delivery::Gone);
    }

    #[test]
    fn a_fixed_link_holds_its_mode_and_takes_sets_quietly() {
        let link = FocusLink::fixed(true);
        assert!(*link.mode().borrow());
        assert_eq!(link.set(false), Delivery::Gone, "no task to carry it out");
        assert!(*link.mode().borrow());
    }
}
