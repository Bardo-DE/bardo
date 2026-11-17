//! What can happen to the connection, and what the person can ask of it.

use key_remap_proto::Status;
use serde::{Deserialize, Serialize};

use crate::keyboard::{ShellFocus, UserOverrides};

/// What a person can ask of the key remap daemon from outside the daemon (`bardo-shell key_remap retry` and `resume`):
/// the part of [`KeyRemapCommand`] that crosses the IPC wire.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum KeyRemapAsk {
    /// Reset the unit's failure and start it again.
    Retry,
    /// End the pause the escape chord caused.
    Resume,
}

/// What the key remap daemon service can be told: a person's [`KeyRemapAsk`], and the settings' overrides.
/// Not on the wire: the overrides come only from `keyboard.overrides` in the settings file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KeyRemapCommand {
    /// Reset the unit's failure and start it again.
    Retry,
    /// End the pause the escape chord caused.
    Resume,
    /// The person's per-app profiles from the settings; replaces the ones the service has and
    /// sends the focus again when the window's profile changes.
    Overrides(UserOverrides),
    /// Whether the shell's own surfaces hold the keyboard: they do not appear as toplevels, so
    /// the daemon says so, and the profile is `keymap-aware` while they do.
    ShellFocus(ShellFocus),
}

impl From<KeyRemapAsk> for KeyRemapCommand {
    fn from(ask: KeyRemapAsk) -> Self {
        match ask {
            KeyRemapAsk::Retry => KeyRemapCommand::Retry,
            KeyRemapAsk::Resume => KeyRemapCommand::Resume,
        }
    }
}

/// The systemd unit's state, as the service reads it from the manager.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum UnitState {
    Active,
    /// Starting, including systemd's automatic restart delay.
    Activating,
    Failed,
    /// Stopped without a failure.
    Inactive,
    /// The manager has no such unit.
    NotFound,
}

/// One fact about the key remap daemon, from the socket or from systemd.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KeyRemapEvent {
    /// The first packet on a new connection.
    Hello(Status),
    /// The key remap daemon's status changed.
    Status(Status),
    /// The socket closed.
    Disconnected,
    UnitState(UnitState),
}
