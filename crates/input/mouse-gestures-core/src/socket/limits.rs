//! What the socket allows a subscriber, and when it stops complaining. Every bound is against a
//! subscriber that is broken or hostile: it is refused or cut, and never allowed to wedge the
//! thread that serves layer-shell-host. (The per-pass bounds of the thread's own loop are the
//! daemon's.)

use seat_guard::{COMPLAINT_INTERVAL, Voice};
use std::time::Instant;

/// Subscribers connected at once. layer-shell-host is the only one in practice; a few more cover a
/// restart overlapping its predecessor and tools such as a stream viewer.
pub const MAX_CLIENTS: usize = 8;

/// The last time a refusal was logged, so a subscriber that keeps knocking cannot fill the log.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(super) struct Complaints {
    last: Option<Instant>,
}

impl Complaints {
    /// Notes a refusal at `now`: spoken the first time and then once per `COMPLAINT_INTERVAL`.
    pub(super) fn note(self, now: Instant) -> (Self, Voice) {
        match self.last {
            Some(at) if now.saturating_duration_since(at) < COMPLAINT_INTERVAL => {
                (self, Voice::Silent)
            }
            _ => (Self { last: Some(now) }, Voice::Speak),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn a_refusal_is_logged_once_an_interval() {
        let t0 = Instant::now();
        let (c, first) = Complaints::default().note(t0);
        assert_eq!(first, Voice::Speak);
        let (c, again) = c.note(t0 + Duration::from_secs(1));
        assert_eq!(again, Voice::Silent);
        let (_, later) = c.note(t0 + COMPLAINT_INTERVAL);
        assert_eq!(later, Voice::Speak);
    }
}
