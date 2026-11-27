//! Who is calling `org.bardo.Compositor1`, proven on the bus thread before a call reaches the
//! event loop (CASW-05). The proof is the caller-identity library's: the sender's pinned
//! process, its image, and its unit as the unit manager states it, judged against the shipped
//! caller table. A cgroup name alone proves nothing, so none is read here.

use super::connect_ask::{NOT_THE_CALLER, Proven};
use caller_identity_dbus::caller_identity_core::{
    NotHeldError, ProcFs, Proof, Requirement, RoleName, Standing,
};
use caller_identity_dbus::caller_identity_linux::RealProc;
use caller_identity_dbus::{BusPeers, ManagerLink, UnitSource, WhoError};
use caller_roles::{AGENT_TIER, PORTAL, SHELL};
use compositor_core::policy::ConnectionRole;
use std::fmt::Debug;
use std::pin::Pin;
use zbus::fdo;

/// The standing of a sender, as a future the bus thread awaits.
pub type WhoFuture<'a> = Pin<Box<dyn Future<Output = Result<Standing, WhoError>> + Send + 'a>>;

/// The seam that says who a bus sender is.
///
/// The real one is [`BusPeers`]; a test makes a sender stand as a chosen caller
/// (`BusPeers::introduce`), and a test-only build can name units for jailed peers
/// (`DebugPeerUnits`).
pub trait CallerCheck: Debug + Send + Sync + 'static {
    /// The standing of the unique bus name `sender`. An error is an unknown caller.
    fn who<'a>(&'a self, sender: &'a str) -> WhoFuture<'a>;
}

impl<P, U> CallerCheck for BusPeers<P, U>
where
    P: ProcFs + Debug + 'static,
    U: UnitSource + Debug + 'static,
{
    fn who<'a>(&'a self, sender: &'a str) -> WhoFuture<'a> {
        Box::pin(BusPeers::who(self, sender))
    }
}

/// The real identifier: the system's `/proc`, the unit manager on the bus, the probe run on the
/// blocking pool of the tokio runtime the bus thread owns.
pub type DesktopCallers = BusPeers<RealProc, ManagerLink<RealProc>>;

/// The role a caller must hold to be handed a connection with `role`: the shell's for the
/// shell, the agent tier's for the agent tier, the portal's for the portal, none for any other
/// kind of connection.
///
/// ```
/// use compositor_core::policy::ConnectionRole;
/// use compositor_ipc::dbus::role_for_connection;
/// use caller_roles::{PORTAL, SHELL};
///
/// assert_eq!(role_for_connection(&ConnectionRole::Shell), Some(SHELL));
/// assert_eq!(role_for_connection(&ConnectionRole::Portal), Some(PORTAL));
/// assert!(role_for_connection(&ConnectionRole::Public).is_none());
/// ```
#[must_use]
pub fn role_for_connection(role: &ConnectionRole) -> Option<RoleName> {
    match role {
        ConnectionRole::Shell => Some(SHELL),
        ConnectionRole::Agent => Some(AGENT_TIER),
        ConnectionRole::Portal => Some(PORTAL),
        _ => None,
    }
}

/// Proves that `sender` holds one of `roles`, all the way: the image and the unit as the manager
/// states it (`Proof::Full`), with either kind of pin (a bus may not offer a process descriptor).
///
/// A caller that is not one of them is refused with the one fixed sentence, and the refusal is
/// logged once with the roles asked and the standing found.
///
/// # Errors
///
/// Returns the D-Bus error `NotTheCaller` when `sender` is unknown, cannot be looked up, or does
/// not prove one of `roles` in full.
pub async fn require(
    check: &dyn CallerCheck,
    sender: Option<&str>,
    roles: &[RoleName],
) -> fdo::Result<Proven> {
    let Some(sender) = sender else {
        log_refusal(roles, &"no sender");
        return Err(not_the_caller());
    };
    let standing = match check.who(sender).await {
        Ok(standing) => standing,
        Err(error) => {
            log_refusal(roles, &error);
            return Err(not_the_caller());
        }
    };
    match admitted(&standing, roles) {
        Ok(()) => Proven::of(&standing).ok_or_else(not_the_caller),
        Err(refused) => {
            log_refusal(roles, &found(&standing, refused));
            Err(not_the_caller())
        }
    }
}

/// Whether `standing` holds one of `roles` all the way (`Proof::Full`) and, only in a build with
/// feature `dev-rows`, as a developer row (`Proof::Dev`) of one of them. The developer step is
/// exactly that proof ([`Requirement::dev`]), so a service known by its image or its control
/// group alone is refused there as it is everywhere.
fn admitted(standing: &Standing, roles: &[RoleName]) -> Result<(), NotHeldError> {
    let need: Vec<Requirement> = roles
        .iter()
        .flat_map(|role| {
            [
                Some(Requirement::new(role.clone(), Proof::Full)),
                dev_way_in(role),
            ]
        })
        .flatten()
        .collect();
    standing.holds(&need).map(|_| ())
}

/// The developer row's way in as `role`, in a build with `dev-rows`.
#[cfg(feature = "dev-rows")]
fn dev_way_in(role: &RoleName) -> Option<Requirement> {
    Some(Requirement::dev(role.clone()))
}

/// No developer rows in this build: no way in.
#[cfg(not(feature = "dev-rows"))]
fn dev_way_in(_role: &RoleName) -> Option<Requirement> {
    None
}

/// What the log says was found: the standing and why it did not do.
fn found(standing: &Standing, refused: NotHeldError) -> String {
    format!("{standing:?} ({refused})")
}

fn log_refusal(roles: &[RoleName], found: &dyn std::fmt::Display) {
    tracing::warn!(
        asked = ?roles,
        found = %found,
        "a call to compositor over the bus was refused: the caller does not hold a role the method answers"
    );
}

fn not_the_caller() -> fdo::Error {
    fdo::Error::AccessDenied(NOT_THE_CALLER.to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;
    use caller_identity_dbus::caller_identity_core::{AppKey, Pin as Pinning};

    fn shell_at(proof: Proof) -> Standing {
        Standing::Service {
            app: AppKey::new("org.bardo.Shell").expect("an app"),
            role: SHELL,
            proof,
            pin: Pinning::Reopened,
        }
    }

    #[test]
    fn a_role_is_held_all_the_way_or_not_at_all() {
        let asked = [SHELL];
        admitted(&shell_at(Proof::Full), &asked).unwrap();
        assert!(admitted(&shell_at(Proof::Exe), &asked).is_err());
        assert!(admitted(&shell_at(Proof::Cgroup), &asked).is_err());
        assert!(admitted(&shell_at(Proof::Full), &[PORTAL]).is_err());
    }

    /// A developer row's service is let in for its own role and only that proof; the other
    /// refusals stand, and the real service is let in by its real requirement.
    #[cfg(feature = "dev-rows")]
    #[test]
    fn a_developer_row_is_let_in_for_its_own_role_only() {
        admitted(&shell_at(Proof::Dev), &[SHELL]).unwrap();
        admitted(&shell_at(Proof::Dev), &[AGENT_TIER, SHELL]).unwrap();
        assert!(admitted(&shell_at(Proof::Dev), &[PORTAL]).is_err());
        assert!(admitted(&shell_at(Proof::Exe), &[SHELL]).is_err());
        assert!(admitted(&shell_at(Proof::Cgroup), &[SHELL]).is_err());
    }
}
