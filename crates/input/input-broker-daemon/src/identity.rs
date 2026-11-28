//! Who is calling.
//!
//! The kernel names the process behind the socket (a pidfd, so a recycled process id cannot pass
//! for another program); `caller_identity` judges it against the root-owned table of desktop programs;
//! `input_broker_core::policy` decides what that program may have.

// Test seams: never in a shipped build.
pub mod caller_identity_authority;
#[cfg(any(test, feature = "fake"))]
pub mod fake;
pub mod seat;

use caller_identity_core::{ProbeError, Uid};
use caller_identity_linux::{LoadError, PinError};
use input_broker_core::identity::Caller;
use rustix::io::Errno;
use rustix::net::sockopt::socket_peercred;
use std::os::fd::OwnedFd;

/// Why a caller could not be identified.
#[derive(Debug, thiserror::Error)]
pub enum IdentifyError {
    /// The kernel would not name the process (an old kernel, or it is gone).
    #[error("the caller could not be pinned: {0}")]
    Pin(#[source] PinError),
    /// Its facts could not be read.
    #[error("the caller's facts could not be read: {0}")]
    Facts(#[source] ProbeError),
    /// The table of desktop programs could not be read.
    #[error("the table of desktop programs could not be read: {0}")]
    Table(#[source] LoadError),
    /// The kernel would not name the user behind the socket.
    #[error("the user behind the socket could not be named: {0}")]
    Peer(#[source] Errno),
}

/// Names the process behind a connection.
pub trait Authority: Send + Sync + 'static {
    /// Identifies the peer of `conn`. It may take for ever: the peer controls what its own
    /// `/proc` entries and image file wait on, so the daemon calls it on a thread of its own.
    ///
    /// # Errors
    ///
    /// Returns an [`IdentifyError`] when the peer's credentials, its pidfd or its `/proc` entries
    /// cannot be read.
    fn identify(&self, conn: &OwnedFd) -> Result<Caller, IdentifyError>;

    /// The user the kernel names for the peer of `conn`, from the socket alone: no `/proc` read, so
    /// it cannot wait on the peer. The daemon counts a connection by this before it identifies it.
    ///
    /// # Errors
    ///
    /// Returns [`IdentifyError::Peer`] when the socket's peer credentials cannot be read.
    fn peer_uid(&self, conn: &OwnedFd) -> Result<Uid, IdentifyError> {
        socket_peercred(conn)
            .map(|cred| Uid::new(cred.uid.as_raw()))
            .map_err(IdentifyError::Peer)
    }
}
