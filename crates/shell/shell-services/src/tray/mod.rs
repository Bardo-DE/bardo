//! The status tray: an SNI host with dbusmenu (system-tray 0.8.9: SNI host + dbusmenu; registers a
//! watcher if none).
//!
//! `TrayService` (in `shell_model::tray`) and its backend do the effect (a
//! `system_tray::client::Client` on the session bus). Neither reaches `dioxus`, `ds` or
//! `layer-shell-host` (`scripts/check-boundary.sh`): the bar reads this module only through
//! `shell_model::tray`, via `use_service::<TrayService>`; the art of an icon is the bar's own
//! (`shell_bar::bar::tray_icon`).

mod backend;

use crate::env::Env;
use crate::error::ServiceError;
use crate::service::Run;
use shell_model::service::ServiceIo;
use shell_model::tray::TrayService;

impl Run for TrayService {
    type Deps = ();

    fn run(
        env: &Env,
        (): (),
        io: ServiceIo<Self>,
    ) -> impl Future<Output = Result<(), ServiceError>> + Send + 'static {
        let bus = env.buses.session.clone();
        async move {
            backend::run(bus, io.state, io.commands).await;
            Ok(())
        }
    }
}
