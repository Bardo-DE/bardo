//! The role names of the desktop programs that lower layers must name.
//!
//! A service says who may call it with a [`RoleName`] and the caller table
//! (`dist/desktop-callers.toml`) says which program has that role. The service that owns a role
//! defines its constant in its own lowest crate, where the layers allow it: the touchpad daemon's
//! role in `input-broker-wire`, the accounts service's in `account-core`, and so on. The five
//! roles here are different: the compositor, the input daemons, the portal, the updater and the
//! menu library all name the shell, and the compositor names the portal, its frontend, the agent
//! tier and Settings, yet each owner sits in a layer above some of the services that name it, which
//! may not depend on it. They are therefore defined once here, below every layer that needs them.
//!
//! Every constant is checked against the shipped table by this crate's test.

use caller_identity_core::RoleName;

/// The shell (`shell`): draws the desktop and asks the person on behalf of other programs.
pub const SHELL: RoleName = RoleName::from_static("shell");
/// The desktop's portal backend (`portal`): asks the shell, is handed an input socket, captures the screen.
pub const PORTAL: RoleName = RoleName::from_static("portal");
/// The portal frontend (`xdg-desktop-portal`): the only caller of the portal backend.
pub const PORTAL_FRONTEND: RoleName = RoleName::from_static("portal_frontend");
/// The agent tier (`computer_use_daemon`): the process that drives other programs for an assistant.
pub const AGENT_TIER: RoleName = RoleName::from_static("agent_tier");
/// The Settings app.
pub const SETTINGS: RoleName = RoleName::from_static("settings");
