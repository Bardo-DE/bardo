//! The client side of one `Open` fd: frames out, events in. `inference-client` re-exports it and
//! each transport supplies its own session; `account-fake` supplies a scripted one.

use crate::event::{ClientFrame, InferEvent};
use std::sync::Arc;

/// Why a session could not carry a frame.
///
/// Two faults are equal when they are the same failure: the same kind, with the same words for a
/// malformed frame and the same kind of operating system error for a broken socket.
#[derive(Debug, Clone, thiserror::Error)]
pub enum SessionError {
    /// The other side closed the session.
    #[error("session closed")]
    Closed,
    /// The session's socket failed under a read or a write. The error is shared so the fault can
    /// be cloned.
    #[error("session socket failed")]
    Broken(#[source] Arc<std::io::Error>),
    /// The other side sent something that is not porter's protocol.
    #[error("malformed frame: {0}")]
    Malformed(String),
}

impl PartialEq for SessionError {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Closed, Self::Closed) => true,
            (Self::Broken(a), Self::Broken(b)) => a.kind() == b.kind(),
            (Self::Malformed(a), Self::Malformed(b)) => a == b,
            _ => false,
        }
    }
}

impl Eq for SessionError {}

/// One open inference session, pinned to the model the route chose when it was opened.
pub trait InferSession: Send {
    /// Writes one frame (a request, `Cancel`, an audio frame, `EndOfAudio`).
    fn send(&mut self, frame: ClientFrame)
    -> impl Future<Output = Result<(), SessionError>> + Send;

    /// Writes one frame with descriptors (memfds) riding on it as `SCM_RIGHTS`: a frame names one
    /// by its index in `attachments` (`ImageSource::Attached`). A session that carries none
    /// refuses a non-empty list. Unix only: a descriptor is not a thing elsewhere.
    #[cfg(unix)]
    fn send_attached(
        &mut self,
        frame: ClientFrame,
        attachments: Vec<std::os::fd::OwnedFd>,
    ) -> impl Future<Output = Result<(), SessionError>> + Send {
        async move {
            if attachments.is_empty() {
                self.send(frame).await
            } else {
                Err(SessionError::Malformed(
                    "this session carries no attachments".to_owned(),
                ))
            }
        }
    }

    /// The next event; `Err(Closed)` after the daemon ends the session.
    fn next(&mut self) -> impl Future<Output = Result<InferEvent, SessionError>> + Send;
}
