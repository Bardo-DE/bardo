//! The eight services Settings talks to, and the role the owner of each one's bus name must hold.

use account_wire::names::{ACCOUNTS_BUS, INFERENCE_BUS};
use caller_identity_dbus::caller_identity_core::{Proof, RoleName};
use caller_roles::{PORTAL, SHELL};
use compositor_dbus::BUS_NAME as COMPOSITOR_BUS;
use memory_dbus::MEMORY_BUS;
use portal_dbus::BUS_NAME as PORTAL_BUS;
use shell_dbus::DISPLAYS_BUS;
use sync_bus::SYNC_BUS;
use updater_dbus::BUS_NAME as UPDATES_BUS;

/// The proof a service's owner must show.
///
/// All eight run as units, so the unit manager confirms the unit: a process that merely sits in the
/// right cgroup, or runs the right image from a unit the person can edit, does not pass.
pub const SERVICE_PROOF: Proof = Proof::Full;

/// The role the owner of `name` must hold; none for a name that is not one of the eight (such a
/// name is never vouched for).
#[must_use]
pub fn required_role(name: &str) -> Option<RoleName> {
    let role = match name {
        ACCOUNTS_BUS => account_wire::roles::ACCOUNTS,
        SYNC_BUS => sync_bus::roles::SYNC,
        INFERENCE_BUS => account_wire::roles::INFERENCE,
        UPDATES_BUS => updater_dbus::roles::UPDATER,
        MEMORY_BUS => memory_dbus::roles::MEMORY,
        COMPOSITOR_BUS => compositor_dbus::roles::COMPOSITOR,
        PORTAL_BUS => PORTAL,
        DISPLAYS_BUS => SHELL,
        _ => return None,
    };
    Some(role)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_service_names_the_role_of_its_program() {
        let wanted = [
            (ACCOUNTS_BUS, "accounts"),
            (SYNC_BUS, "sync"),
            (INFERENCE_BUS, "inference"),
            (UPDATES_BUS, "updater"),
            (MEMORY_BUS, "memory"),
            (COMPOSITOR_BUS, "compositor"),
            (PORTAL_BUS, "portal"),
            (DISPLAYS_BUS, "shell"),
        ];
        for (name, role) in wanted {
            let held = required_role(name).map(|role| role.to_string());
            assert_eq!(held.as_deref(), Some(role), "{name}");
        }
    }

    #[test]
    fn a_name_that_is_not_a_service_has_no_role() {
        assert_eq!(required_role("org.bardo.Settings"), None);
        assert_eq!(required_role("org.example.Accounts1"), None);
        assert_eq!(required_role(""), None);
    }
}
