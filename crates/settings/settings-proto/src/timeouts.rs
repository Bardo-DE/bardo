//! How long a client waits on a live module.

use base_time::deadline::Deadline;

/// How long each call to a live module may take before it is a timeout error.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Timeouts {
    /// `Describe`, `Get`, `Snapshot` and the subscription: the module reads its own state.
    pub read: Deadline,
    /// `Set`, including running an action key: the module may ask a provider, or the person.
    pub action: Deadline,
}

impl Timeouts {
    /// Timeouts of `read` for reads and `action` for a `Set`.
    #[must_use]
    pub const fn new(read: Deadline, action: Deadline) -> Timeouts {
        Timeouts { read, action }
    }
}

impl Default for Timeouts {
    fn default() -> Timeouts {
        Timeouts::new(Deadline::READ, Deadline::ACTION)
    }
}
