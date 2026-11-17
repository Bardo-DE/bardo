//! Night Shift: the compositor's software night light, asked for by the person.
//!
//! The compositor warms the screen from `[night_light]` in `~/.config/compositor/settings.toml`; this service reads that
//! table and writes it when the person turns Night Shift on until tomorrow, turns it off or
//! moves the warmth, and keeps the manual turn-on's end in `night-shift.json` in the state
//! directory so a restart does not leave the screen warm all day. Everything else in the compositor's
//! file is left as it is (the table is merged key by key); an edit made elsewhere (Settings) is
//! read back through a directory watch.
//!
//! Where bardo-shell warms the monitors itself over DDC/CI (`display.night_warmth`), the shell tells this
//! service (`NightShiftCommand::Backend`) and it lists those monitors' connectors in the compositor's
//! `night_light.skip_outputs`, following the displays service as monitors come and go; the compositor
//! keeps warming every other output. Entries of that list that name a display by its identity
//! (`edid:...`) are the person's and stay.

mod backend;

use shell_model::displays::Displays;
use tokio::sync::watch;

use crate::env::Env;
use crate::error::ServiceError;
use crate::service::Run;
use shell_model::night_shift::NightShiftService;
use shell_model::service::ServiceIo;

impl Run for NightShiftService {
    type Kind = Self;
    /// The displays, whose DDC monitors the compositor is told to leave alone where bardo-shell warms them.
    type Deps = watch::Receiver<Displays>;

    fn run(
        env: &Env,
        deps: watch::Receiver<Displays>,
        io: ServiceIo<Self>,
    ) -> impl Future<Output = Result<(), ServiceError>> + Send + 'static {
        let compositor = env.dirs.config.join("bardo").join("compositor");
        let state = env.dirs.sill_state();
        let clock = std::sync::Arc::clone(&env.clock);
        async move {
            backend::run(compositor, state, clock, deps, io).await;
            Ok(())
        }
    }
}
