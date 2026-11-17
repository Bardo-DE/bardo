//! The password check behind bardo-shell's own lock screen (design/20 §1.9).
//!
//! The password check behind bardo-shell's own lock screen (design/20 §1.9). The lock surface sends the
//! typed secret ([`AuthCommand::Submit`]); the service runs it through an `Authenticator` off the
//! UI thread, counts failures through the pure `gate` machine (one check at a time; no lockout),
//! and publishes [`Auth`], whose `AuthPhase::Accepted` is what the lock surfaces unlock on (the
//! daemon as a backstop). The polkit prompt does not come through here: polkit's own helper runs
//! PAM for it (`crate::polkit_agent`).
//!
//! How lockers authenticate without root (researched): PAM's `pam_unix` sees a caller that is not
//! root and runs the setuid `unix_chkpwd` helper, which checks the caller's
//! own password against `/etc/shadow` and nobody else's. swaylock (MIT) runs PAM in a forked
//! child that it talks to over a pipe (service `swaylock`, whose file is `auth include login`);
//! hyprlock runs it on a thread (service `hyprlock`); a greeter's locker can call `pam-client`
//! on tokio's blocking pool, then `acct_mgmt`. bardo-shell takes swaylock's
//! shape: [`PamHelper`] runs `bardo-shell auth-check` as a child per attempt, which calls PAM through
//! the safe `nonstick` crate ([`backend::pam::check`]), so a module that hangs (a network
//! directory, a
//! fingerprint reader) is killed with its child instead of wedging a thread, and the lock
//! surface's process never holds a PAM handle. The PAM service is `bardo-shell` when
//! `/etc/pam.d/bardo-shell` exists (`dist/pam.d/bardo-shell`: `auth include login`, as swaylock's), else
//! `login` ([`backend::pam::service`]).
//!
//! Nothing here runs PAM until a [`AuthCommand::Submit`] arrives, and every test drives a
//! `FakeAuthenticator` (the [`Env`]'s authenticator); no test calls PAM with anyone's real
//! password.

mod backend;

use std::sync::Arc;

use backend::AccountPlace;

use crate::env::Env;
use crate::error::ServiceError;
use crate::service::Run;
use shell_model::auth::AuthService;
use shell_model::service::ServiceIo;

pub use backend::{PamHelper, pam};

impl Run for AuthService {
    type Kind = Self;
    type Deps = ();

    fn run(
        env: &Env,
        (): (),
        io: ServiceIo<Self>,
    ) -> impl Future<Output = Result<(), ServiceError>> + Send + 'static {
        let account = AccountPlace {
            system: env.buses.system.clone(),
            home: env.dirs.home.clone(),
        };
        let passwd = Arc::clone(&env.passwd);
        let authenticator = Arc::clone(&env.authenticator);
        async move {
            backend::run(io.state, io.commands, account, passwd, authenticator).await;
            Ok(())
        }
    }
}
