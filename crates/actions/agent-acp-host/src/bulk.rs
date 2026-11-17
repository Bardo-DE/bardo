//! Where the agent sandbox's types meet action-core's.
//!
//! The agent sandbox owns its sandbox-state and network types (it depends on no actions crate; the
//! path is its own, shared), and these are the conversions at the edge.

use permission_core::{CannotSandboxError, SandboxState};
use sandbox::{
    CannotSandboxError as ShellCannot, Network, NetworkMode, SandboxState as ShellState,
};

/// Why the agent sandbox cannot confine, as action-core words it.
#[must_use]
pub const fn cannot(why: ShellCannot) -> CannotSandboxError {
    match why {
        ShellCannot::NotInstalled => CannotSandboxError::NotInstalled,
        ShellCannot::NamespacesDenied => CannotSandboxError::NamespacesDenied,
        ShellCannot::RestrictedByAppArmor => CannotSandboxError::RestrictedByAppArmor,
        ShellCannot::Unsupported => CannotSandboxError::Unsupported,
        ShellCannot::BadWorkingDir => CannotSandboxError::BadWorkingDir,
        ShellCannot::LimitsUnavailable => CannotSandboxError::LimitsUnavailable,
        ShellCannot::SeccompUnavailable => CannotSandboxError::SeccompUnavailable,
        ShellCannot::BadLimits => CannotSandboxError::BadLimits,
    }
}

/// The agent sandbox's sandbox state as action-core's.
#[must_use]
pub const fn state(state: ShellState) -> SandboxState {
    match state {
        ShellState::Ready => SandboxState::Ready,
        ShellState::Cannot(why) => SandboxState::Cannot(cannot(why)),
    }
}

/// Whether a terminal's sandbox has a network.
#[must_use]
pub const fn access(network: Network) -> bool {
    matches!(network, Network::Host)
}

/// Whether an agent process's sandbox has a network: anything but none can reach something. An
/// allowlist reaches the named hosts: open, as far as a command is concerned.
#[must_use]
pub const fn mode_access(mode: NetworkMode) -> bool {
    !matches!(mode, NetworkMode::None)
}
