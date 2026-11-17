//! The dock's pinned apps, the `[[dock]]` rows of `~/.config/bardo/shell/layout.toml`.
//!
//! Membership and order are data in the program's layout file (design/28 §5.6). The old
//! `dock.json` is read once into it and left in place.

mod backend;

use shell_model::apps::AppIndex;
use shell_model::pins::DockPins;
use tokio::sync::watch;

use crate::env::Env;
use crate::error::ServiceError;
use crate::service::Run;
use shell_model::pins::PinsService;
use shell_model::service::ServiceIo;

impl Run for PinsService {
    type Kind = Self;
    /// The installed apps, which a fresh account's dock starts from.
    type Deps = watch::Receiver<AppIndex>;

    fn initial(env: &Env, _apps: &watch::Receiver<AppIndex>) -> DockPins {
        backend::load(&env.dirs.sill_config())
    }

    fn run(
        env: &Env,
        deps: watch::Receiver<AppIndex>,
        io: ServiceIo<Self>,
    ) -> impl Future<Output = Result<(), ServiceError>> + Send + 'static {
        let dir = env.dirs.sill_config();
        async move {
            backend::run(dir, io.state, io.commands, deps).await;
            Ok(())
        }
    }
}
