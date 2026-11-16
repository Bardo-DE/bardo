//! The session state's pure transition.

use super::{Session, SessionEvent};

/// `session` after `event`.
#[must_use]
pub fn step(session: Session, event: SessionEvent) -> Session {
    let _ = session;
    match event {
        SessionEvent::Probed(probed) => probed,
        SessionEvent::Gone => Session::default(),
    }
}
