//! bardo-shell serves each display's brightness on the session bus.
//!
//! Settings shows a slider per display and follows the keys and the control centre:
//! `org.bardo.Displays1` at `/org/bardo/Displays1`, a small interface over what the displays
//! service already routes (logind for built-in panels, DDC/CI for monitors).
//!
//! - `Brightness() -> a(su)`: every output whose brightness can be read, with its percentage
//!   (0 to 100); an output with nothing to read is not listed.
//! - `SetBrightness(s output, u percent)`: sets that output's own brightness. Only Settings may
//!   call it (the program the table of the desktop's own programs names as `settings`, owning
//!   `org.bardo.Settings`, [`crate::callers`]); anyone else is refused (`AccessDenied`). An
//!   output that is not listed, or a percentage above 100, is `InvalidArgs`.
//! - `Changed(a(su))`: the same list, sent whenever it changes (a key, the control centre, a
//!   monitor adjusted on itself while a view follows).
//! - **The name.** `org.bardo.Displays1`, claimed at start unless [`Env::claims`](crate::env) is
//!   `Passive` (a test's `start` never claims it). Reading is open to any program.

mod object;
mod run;

use shell_model::displays::{Displays, DisplaysCommand};
use tokio::sync::watch;

use crate::env::Env;
use crate::error::ServiceError;
use crate::service::Run;
use shell_model::serve::DisplaysBusService;
use shell_model::service::Control;
use shell_model::service::ServiceIo;

/// What the service reads and drives: the displays it lists and the displays service it tells.
#[derive(Debug, Clone)]
pub struct DisplaysBusLinks {
    pub displays: watch::Receiver<Displays>,
    pub control: Control<DisplaysCommand>,
}

impl Run for DisplaysBusService {
    type Kind = Self;
    type Deps = DisplaysBusLinks;

    fn run(
        env: &Env,
        deps: DisplaysBusLinks,
        io: ServiceIo<Self>,
    ) -> impl Future<Output = Result<(), ServiceError>> + Send + 'static {
        let bus = env.buses.session.clone();
        let claims = env.claims;
        let callers = std::sync::Arc::clone(&env.callers);
        async move {
            run::run(bus, claims, callers, deps, io).await;
            Ok(())
        }
    }
}
