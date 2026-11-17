//! Starting apps and the desktop's one terminal opener (`backend/terminal/`).
//!
//! Starting covers the activation token, `DBusActivatable` via org.freedesktop.Application.Activate,
//! the Terminal=true wrapper, and `systemd-run --user --scope --collect
//! --unit=app-bardo-<id>-<rand>.scope` (fallback process group).
//!
//! The activation token comes from layer-shell-host (`SurfaceHandle::activation_token`) in the
//! daemon's `shell_app::launch`, which then sends a [`LaunchCommand`] here. [`LaunchService`] prefers
//! `org.freedesktop.Application.Activate` for a `DBusActivatable` entry (`dbus.rs`); otherwise
//! it plans the argument vector (`plan.rs`, pure) and runs it inside a transient
//! `systemd-run --user --scope`, wrapped in the machine's terminal (`terminal/`) when the
//! entry asks for one, tracking the scope until it exits (`run.rs`).

mod backend;
#[cfg(test)]
mod tests;

use backend::Reach;

use crate::env::Env;
use crate::error::ServiceError;
use crate::service::Run;
use shell_model::launch::LaunchService;
use shell_model::service::ServiceIo;

pub use backend::{LaunchLinks, file_uri, monotonic_now, open_argv, reveal_folder};
pub(crate) use backend::{
    Manager, TerminalCommand, TerminalSearch, open_in_terminal, user_manager,
};

impl Run for LaunchService {
    type Kind = Self;
    type Deps = LaunchLinks;

    fn run(
        env: &Env,
        deps: Self::Deps,
        io: ServiceIo<Self>,
    ) -> impl Future<Output = Result<(), ServiceError>> + Send + 'static {
        let reach = Reach::of(env);
        async move {
            backend::run(reach, deps, io.state, io.commands).await;
            Ok(())
        }
    }
}
