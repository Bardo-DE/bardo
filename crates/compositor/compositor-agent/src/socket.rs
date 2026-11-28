//! The hand-out of privileged connections (C20, I7): `org.bardo.Compositor1.Connect(role)`
//! answers with a connected socket whose client is inserted with that role, after checking
//! the role the caller proved. The only place a `Shell`, `Agent` or `Portal` connection is made
//! (B14).
//!
//! This crate decides and makes the socket pair; the composition root inserts the server end
//! into the Wayland display as a client of the granted role (`compositor_app::connect`), because
//! only it can build a client's data (its key, its gone queue). Frozen at P2-Z; filled by
//! P2-Q2.

use caller_identity_core::{Proof, Requirement, RoleName, Standing};
use caller_roles::{AGENT_TIER, PORTAL, SHELL};
use compositor_core::policy::ConnectionRole;
use std::os::fd::OwnedFd;
use std::os::unix::net::UnixStream;

/// A granted connection (the new connection the caller gets): a socket pair and the role its client is inserted with.
#[derive(Debug)]
pub struct Handed {
    /// The role the client has (I7).
    pub role: ConnectionRole,
    /// The server end, for `Display::insert_client`.
    pub server: UnixStream,
    /// The client end, returned to the caller over D-Bus.
    pub client: OwnedFd,
}

/// Why a hand-out was refused.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RefuseWhy {
    /// The caller holds no role the compositor hands a connection to.
    UnknownUnit,
    /// The caller's role may not have this kind of connection.
    RoleNotAllowed,
    /// The socket pair could not be made.
    NoSocket,
}

/// Hands a caller whose proven `standing` holds a role a connection with `wanted`, if that role is
/// the one `standing` stands for.
///
/// The connection is a socket pair, the role kept with it. `standing` is what the bus thread proved
/// of the caller (`compositor_ipc::dbus`), never anything the caller said; this check is the
/// second line should a caller of this function ask for more than the proof gave. The caller of
/// this function inserts the server end into the display.
///
/// # Errors
///
/// Returns [`RefuseWhy::UnknownUnit`] when `standing` stands for no connection role,
/// [`RefuseWhy::RoleNotAllowed`] when it does not stand for `wanted`, and
/// [`RefuseWhy::NoSocket`] when the socket pair cannot be made.
pub fn hand_out(wanted: &ConnectionRole, standing: &Standing) -> Result<Handed, RefuseWhy> {
    match connection_role(standing) {
        None => Err(RefuseWhy::UnknownUnit),
        Some(allowed) if allowed != *wanted => Err(RefuseWhy::RoleNotAllowed),
        Some(allowed) => match UnixStream::pair().and_then(|pair| {
            // The display reads the server end from its event loop: it must never block.
            pair.0.set_nonblocking(true)?;
            Ok(pair)
        }) {
            Ok((server, client)) => Ok(Handed {
                role: allowed,
                server,
                client: OwnedFd::from(client),
            }),
            Err(_) => Err(RefuseWhy::NoSocket),
        },
    }
}

/// The connection role a proven caller's `standing` stands for.
///
/// The shell the shell's, the agent tier the agent's, the portal the portal's, every other role
/// (and any standing that is not a service held in full) none. The role comes from the shipped
/// caller table (the caller-identity library), never from a unit name.
#[must_use]
pub fn connection_role(standing: &Standing) -> Option<ConnectionRole> {
    [
        (SHELL, ConnectionRole::Shell),
        (AGENT_TIER, ConnectionRole::Agent),
        (PORTAL, ConnectionRole::Portal),
    ]
    .into_iter()
    .find(|(role, _)| holds_role(standing, role))
    .map(|(_, connection)| connection)
}

/// Whether `standing` meets the requirement the bus thread made for `role`: the role proved in
/// full (image and unit), and, only in a build with feature `dev-rows`, a developer row's proof.
pub(crate) fn holds_role(standing: &Standing, role: &RoleName) -> bool {
    let need = [
        Requirement::new(role.clone(), Proof::Full),
        Requirement::dev(role.clone()),
    ];
    let wanted = if cfg!(feature = "dev-rows") {
        &need[..]
    } else {
        &need[..1]
    };
    standing.holds(wanted).is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use caller_identity_core::{AppKey, Pin};

    fn role(name: &str) -> RoleName {
        RoleName::new(name).expect("a role name")
    }

    /// A service standing for `held` at `proof`.
    fn standing(held: RoleName, proof: Proof) -> Standing {
        Standing::Service {
            app: AppKey::new("org.bardo.Test").expect("an app"),
            role: held,
            proof,
            pin: Pin::AtConnect,
        }
    }

    fn full(held: RoleName) -> Standing {
        standing(held, Proof::Full)
    }

    #[test]
    fn only_the_shell_the_agent_tier_and_the_portal_stand_for_privileged_roles() {
        assert_eq!(connection_role(&full(SHELL)), Some(ConnectionRole::Shell));
        assert_eq!(
            connection_role(&full(AGENT_TIER)),
            Some(ConnectionRole::Agent)
        );
        assert_eq!(connection_role(&full(PORTAL)), Some(ConnectionRole::Portal));
        for other in ["settings", "compositor", "companion", "shell_helper"] {
            assert_eq!(connection_role(&full(role(other))), None, "{other}");
        }
    }

    #[test]
    fn a_role_held_in_less_than_full_stands_for_nothing() {
        for proof in [Proof::Cgroup, Proof::Exe, Proof::Unit] {
            assert_eq!(connection_role(&standing(SHELL, proof)), None, "{proof:?}");
        }
    }

    /// (case, the role the caller proved, the role asked, what comes back)
    #[test]
    fn a_hand_out_needs_the_role_that_stands_for_the_connection() {
        let cases = [
            (
                "the shell asks for the shell",
                SHELL,
                ConnectionRole::Shell,
                None,
            ),
            (
                "the agent tier asks for the agent tier",
                AGENT_TIER,
                ConnectionRole::Agent,
                None,
            ),
            (
                "the shell asks for the agent tier",
                SHELL,
                ConnectionRole::Agent,
                Some(RefuseWhy::RoleNotAllowed),
            ),
            (
                "the agent tier asks for the shell",
                AGENT_TIER,
                ConnectionRole::Shell,
                Some(RefuseWhy::RoleNotAllowed),
            ),
            (
                "the portal asks for the portal",
                PORTAL,
                ConnectionRole::Portal,
                None,
            ),
            (
                "the portal asks for the shell",
                PORTAL,
                ConnectionRole::Shell,
                Some(RefuseWhy::RoleNotAllowed),
            ),
            (
                "the shell asks for the portal",
                SHELL,
                ConnectionRole::Portal,
                Some(RefuseWhy::RoleNotAllowed),
            ),
            (
                "the settings app asks for the portal",
                role("settings"),
                ConnectionRole::Portal,
                Some(RefuseWhy::UnknownUnit),
            ),
        ];
        for (case, held, wanted, refused) in cases {
            match (hand_out(&wanted, &full(held)), refused) {
                (Ok(handed), None) => assert_eq!(handed.role, wanted, "{case}"),
                (Err(why), Some(want)) => assert_eq!(why, want, "{case}"),
                (other, _) => panic!("{case}: {other:?}"),
            }
        }
    }
}
