//! The session seam's types.

use smithay::backend::session::AsErrno;

/// Why a session refused.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum SessionError {
    /// The open was refused (in a test: the jail is breached; the test fails with exit 5).
    #[error("denied open {0}")]
    DeniedOpen(String),
    /// The session is not active (another VT has it).
    #[error("inactive")]
    Inactive,
    /// The seat could not be opened or was lost (libseat, udev); the text is the adapter's.
    #[error("seat unavailable: {0}")]
    Unavailable(String),
}

impl AsErrno for SessionError {
    fn as_errno(&self) -> Option<i32> {
        match self {
            SessionError::DeniedOpen(_) => Some(rustix::io::Errno::ACCESS.raw_os_error()),
            SessionError::Inactive | SessionError::Unavailable(_) => None,
        }
    }
}
