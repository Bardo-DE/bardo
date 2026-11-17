//! The state types (data only).

use key_remap_proto::Status;
use serde::{Deserialize, Serialize};

/// Where the connection to the key remap daemon stands.
///
/// Failure and absence are states, not a silent default: a machine without the key remap daemon is
/// `NotInstalled`, one not yet heard from is `Unknown`. Crosses the wire as `bardo-shell key-remap
/// status`.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(tag = "kind", content = "v", rename_all = "snake_case")]
pub enum KeyRemapState {
    /// Nothing known yet, or the unit was stopped on purpose.
    #[default]
    Unknown,
    /// The socket is up; `status` is the key remap daemon's latest word on its mode and keyboards.
    Connected { status: Status },
    /// The connection dropped or the unit is starting again; systemd is expected to finish it.
    Restarting,
    /// The unit failed (a crash loop that systemd gave up on).
    Failed,
    /// No key remap daemon unit exists on this machine.
    NotInstalled,
}

/// What the person should be told about the key remap daemon. Everything else, including a restart that
/// recovers, is silent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Attention {
    /// The key remap daemon is down for good: the keyboards are not remapped.
    Failed,
    /// Another remapper holds this keyboard, so the key remap daemon cannot.
    Conflict { device: String },
    /// The escape chord paused remapping; `Resume` ends it.
    Paused,
}
