//! The input broker: the root service that makes the mouse gestures daemon's and the key remap daemon's virtual input devices, and no
//! others.
//!
//! systemd owns the socket and starts the service on the first connection; the service exits by
//! itself after a minute with no one connected. Each connection is one thread, which also identifies
//! its peer, so a peer that stalls the identification stalls only itself. A caller is
//! identified by the kernel (the process behind the socket, held by a pidfd), judged against the
//! root-owned table of desktop programs, and then may ask for exactly the device kinds of its own
//! program. The mouse gestures daemon's devices are proxied: every event passes the ceiling in `input-broker-wire`.
//! The key remap daemon's are made to a fixed shape and handed over as descriptors.
//!
//! The seams are traits with fakes: [`backend::Backend`] (the kernel), [`identity::Authority`] (who
//! is calling), [`identity::SeatSource`] (who is logged in) and [`clock::Uptime`]. The tests drive
//! the whole service over a socket pair with the fakes and touch no device.
#![forbid(unsafe_code)]

pub mod activation;
pub mod admit;
pub mod backend;
pub mod clock;
pub mod daemon;
pub mod identity;
pub mod idle;
pub mod journal;
pub mod link;
pub mod pending;
pub mod policy;
pub mod serve;
pub mod session;
pub mod slots;
mod tally;

use backend::real::UinputBackend;
use base_clock::SystemMonotonic;
use clock::Uptime;
use daemon::{Daemon, DaemonError, LoopConfig};
use identity::caller_identity_authority::CallerIdentityAuthority;
use identity::seat::SeatFile;
use serve::Context;

/// Why the service stopped with an error.
#[derive(Debug, thiserror::Error)]
pub enum RunError {
    /// It was not started by its socket unit.
    #[error("{0}")]
    Activation(#[from] activation::ActivationError),
    /// The accept loop failed.
    #[error("{0}")]
    Daemon(#[from] DaemonError),
}

/// Runs the service on the socket systemd passed in, until it has been idle for a minute.
///
/// # Errors
///
/// Returns a [`RunError`] when no socket was passed in or the service fails.
pub fn run() -> Result<(), RunError> {
    let listener = activation::listener()?;
    let context = Context {
        backend: UinputBackend::default(),
        authority: CallerIdentityAuthority::system(),
        seat: SeatFile::system(),
        clock: Uptime::new(SystemMonotonic),
    };
    Daemon::new(listener, context, LoopConfig::default()).run()?;
    Ok(())
}
