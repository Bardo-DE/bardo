//! The errors of the bus crate, and the one refusal a method returns.

use caller_identity_core::Refused;
use caller_identity_linux::PinError;
use zbus::fdo;

/// A bus peer that could not be identified. A caller that gets this treats the peer as unknown.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum WhoError {
    /// The sender is not a bus name.
    #[error("the sender is not a bus name")]
    BadName(#[source] zbus::names::Error),
    /// The bus did not answer in time.
    #[error("the bus did not answer in time")]
    Timeout,
    /// The bus has no such connection (it left).
    #[error("the sender is no longer on the bus")]
    NoSuchPeer,
    /// The bus could not be asked.
    #[error("the bus could not be asked")]
    Bus(#[source] zbus::Error),
    /// The bus gave no process id or user for the sender.
    #[error("the bus gave no credentials for the sender")]
    NoCredentials,
    /// The bus gave a process handle that could not be duplicated.
    #[error("the sender's process handle could not be duplicated")]
    ProcessHandle(#[source] std::io::Error),
    /// The process could not be held.
    #[error("the process could not be held")]
    Pin(#[from] PinError),
    /// The connection's process changed while it was looked at.
    #[error("the sender changed while it was looked at")]
    Changed,
    /// The work could not be run off the executor.
    #[error("the probe could not be run")]
    Offload(#[source] crate::Cancelled),
}

/// The unit manager's answer about a unit could not be had.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum UnitError {
    /// The manager did not answer in time.
    #[error("the unit manager did not answer in time")]
    Timeout,
    /// The manager does not have the unit loaded.
    #[error("the unit is not loaded")]
    NotLoaded,
    /// The manager could not be asked.
    #[error("the unit manager could not be asked")]
    Bus(#[source] zbus::Error),
    /// The manager's answer is not shaped as expected.
    #[error("the unit manager's answer is malformed: {0}")]
    Malformed(&'static str),
}

impl UnitError {
    /// What the manager's refusal of `GetUnit` means: a unit it does not know is not loaded, and
    /// any other failure is the bus's.
    pub(crate) fn from_get_unit(error: zbus::Error) -> UnitError {
        match &error {
            zbus::Error::MethodError(name, _, _)
                if name.as_str() == "org.freedesktop.systemd1.NoSuchUnit" =>
            {
                UnitError::NotLoaded
            }
            _ => UnitError::Bus(error),
        }
    }
}

/// The refusal a method returns for `refused`: `AccessDenied` with the fixed text of the refusal
/// and nothing about which check failed. (A `From` impl cannot live here: both types are foreign.)
#[must_use]
pub fn access_denied(refused: Refused) -> fdo::Error {
    fdo::Error::AccessDenied(refused.to_string())
}
