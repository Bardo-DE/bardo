//! Launcher entry badges: counts, progress and urgency apps publish over
//! `com.canonical.Unity.LauncherEntry.Update(s app_uri, a{sv})`.
//!
//! [`BadgesService`] and `live` do the effect (a session-bus match rule, zbus 5 on tokio);
//! `model` is the pure reducer. Neither reaches `dioxus`, `ds` or `layer-shell-host`
//! (`scripts/check-boundary.sh`): the bar and dock read this module only through [`Badges`] and
//! [`BadgesCommand`], via `use_service::<BadgesService>`.

mod backend;

use crate::env::Env;
use crate::error::ServiceError;
use crate::service::Run;
use shell_model::badges::BadgesService;
use shell_model::service::ServiceIo;

impl Run for BadgesService {
    type Kind = Self;
    type Deps = ();

    fn run(
        env: &Env,
        (): (),
        io: ServiceIo<Self>,
    ) -> impl Future<Output = Result<(), ServiceError>> + Send + 'static {
        backend::run(env.buses.session.clone(), io.state, io.commands)
    }
}
