//! Where bulkhead's types meet docket-core's: bulkhead owns its sandbox-state and network types
//! (it depends on no docket crate; the path is its own, shared), and these are the conversions at
//! the edge.

use bulkhead::{CannotSandbox as ShellCannot, Network, NetworkMode, SandboxState as ShellState};
use docket_permissions_core::{CannotSandbox, SandboxState};

/// Why bulkhead cannot confine, as docket-core words it.
pub const fn cannot(why: ShellCannot) -> CannotSandbox {
    match why {
        ShellCannot::NotInstalled => CannotSandbox::NotInstalled,
        ShellCannot::NamespacesDenied => CannotSandbox::NamespacesDenied,
        ShellCannot::RestrictedByAppArmor => CannotSandbox::RestrictedByAppArmor,
        ShellCannot::Unsupported => CannotSandbox::Unsupported,
        ShellCannot::BadWorkingDir => CannotSandbox::BadWorkingDir,
        ShellCannot::LimitsUnavailable => CannotSandbox::LimitsUnavailable,
        ShellCannot::SeccompUnavailable => CannotSandbox::SeccompUnavailable,
        ShellCannot::BadLimits => CannotSandbox::BadLimits,
    }
}

/// bulkhead's sandbox state as docket-core's.
pub const fn state(state: ShellState) -> SandboxState {
    match state {
        ShellState::Ready => SandboxState::Ready,
        ShellState::Cannot(why) => SandboxState::Cannot(cannot(why)),
    }
}

/// Whether a terminal's sandbox has a network.
pub const fn access(network: Network) -> bool {
    matches!(network, Network::Host)
}

/// Whether an agent process's sandbox has a network: anything but none can reach something. An
/// allowlist reaches the named hosts: open, as far as a command is concerned.
pub const fn mode_access(mode: NetworkMode) -> bool {
    !matches!(mode, NetworkMode::None)
}
