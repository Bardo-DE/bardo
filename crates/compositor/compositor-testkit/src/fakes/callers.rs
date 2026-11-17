//! Callers of `org.bardo.Compositor1` as the bus thread proved them.
//!
//! The app and the role of the caller-table row they matched.
//!
//! A test says who is calling instead of building a process
//! table; the proof itself (the process, its image, its unit) is `compositor-ipc`'s tests.

use compositor_ipc::dbus::Proven;
use compositor_ipc::dbus::peer::{AppKey, RoleName, roles};

/// The caller `app` holding `role`.
///
/// # Panics
///
/// If `app` is not an app key or `role` is not a role name.
#[must_use]
pub fn proven(app: &str, role: &str) -> Proven {
    Proven::new(
        AppKey::new(app).expect("an app key"),
        RoleName::new(role).expect("a role name"),
    )
}

/// The shell (sill).
///
/// # Panics
///
/// Never in practice: the app key and role it builds are fixed valid literals.
#[must_use]
pub fn shell() -> Proven {
    Proven::new(
        AppKey::new("org.bardo.Shell").expect("an app key"),
        roles::SHELL,
    )
}

/// The agent tier (cuad).
///
/// # Panics
///
/// Never in practice: the app key and role it builds are fixed valid literals.
#[must_use]
pub fn agent_tier() -> Proven {
    Proven::new(
        AppKey::new("org.bardo.Agent").expect("an app key"),
        roles::AGENT_TIER,
    )
}

/// The portal backend (the portal).
///
/// # Panics
///
/// Never in practice: the app key and role it builds are fixed valid literals.
#[must_use]
pub fn portal() -> Proven {
    Proven::new(
        AppKey::new("org.bardo.Portal").expect("an app key"),
        roles::PORTAL,
    )
}

/// Some other desktop program (a role that is no connection's).
///
/// # Panics
///
/// Never in practice: the app key and role it builds are fixed valid literals.
#[must_use]
pub fn companion() -> Proven {
    Proven::new(
        AppKey::new("org.bardo.Companion").expect("an app key"),
        roles::COMPANION,
    )
}
