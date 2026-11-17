//! The lock screen's password check: the account, the attempts, the gate that decides, and the
//! helper's wire.

mod command;
pub mod gate;
mod model;
pub mod protocol;
mod types;

pub use command::{AuthCommand, FakeVerdict};
pub use gate::{Checks, GateIn, GateOut};
pub use model::{Auth, AuthPhase};
pub use types::{
    Account, Attempt, Attempts, AuthFault, AvatarSource, Outcome, PamNote, PamService, RealName,
    UserName, Verdict,
};

/// The lock screen's password check as a service.
///
/// The lock screen's password check as a service, through `env`'s `Authenticator` (PAM
/// through `bardo-shell auth-check` in the daemon, a `FakeAuthenticator` in every test), the account
/// read from `env`'s accounts database, system bus and home. Idle until the first `Submit`: nothing is checked, and
/// no PAM module runs, before that.
#[derive(Debug)]
pub enum AuthService {}

impl crate::service::Service for AuthService {
    const NAME: &'static str = "auth";
    type State = Auth;
    type Command = AuthCommand;
}
