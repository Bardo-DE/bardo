//! The session: logind's power actions and what it currently allows.
//!
//! Every surface that offers
//! them reads this service's state and sends to its control: the bar's system menu, the power
//! menu (design/20 §1.8) and the launcher's System rows. Only this service asks logind.

mod backend;
mod links;
mod quit_apps;

pub use links::SessionLinks;

use std::sync::Arc;

use crate::env::Env;
use crate::error::ServiceError;
use crate::service::Run;
use quit_apps::Scopes;
use shell_model::service::ServiceIo;
use shell_model::session::SessionService;

impl Run for SessionService {
    type Kind = Self;
    type Deps = SessionLinks;

    fn run(
        env: &Env,
        deps: Self::Deps,
        io: ServiceIo<Self>,
    ) -> impl Future<Output = Result<(), ServiceError>> + Send + 'static {
        let bus = env.buses.system.clone();
        let actions = Arc::clone(&env.session_actions);
        let hint = env.session_id.clone();
        let scopes = Scopes {
            runtime: env.dirs.runtime.clone(),
            children: Arc::clone(&env.children),
        };
        async move {
            backend::run(bus, actions, hint, scopes, io.state, io.commands, deps).await;
            Ok(())
        }
    }
}
