//! Displays: Mac-quality defaults derived from what each display reports, nothing
//! vendor-specific, every default overridable in `display.*`.
//!
//! - **Outputs.** Every output's EDID (size, native mode) gives its density and gamut; the
//!   compositor's own output management ([`OutputsRoute`], picked from its globals) lists
//!   them. bardo-shell never decides or sets a scale: the compositor chooses it, bardo-shell shows it.
//! - **Gamut.** The EDID primaries classified as sRGB, Display P3 or wide (`classify`); stored
//!   for colour management, which no compositor offers yet.
//! - **DDC/CI.** External monitors' brightness, contrast, speaker volume, mute, input and RGB
//!   gains (`Controls`) on one worker thread, with debounced writes and cached reads.
//! - **Brightness routing.** Built-in panels through logind, monitors through DDC
//!   (`BrightnessTarget`), on the output under the pointer or all of them.
//! - **On the bus.** Each display's brightness for Settings, `org.bardo.Displays1` (`bus`).
//! - **Monitor speakers** follow the software volume when the default sink is the monitor.
//! - **Night warmth** through the monitors' RGB gains (`warmth_gains`), off by default.
//! - **Font rendering** for other apps: a fontconfig snippet from the densest display.
//!
//! The service is inert until the daemon sends its first [`DisplaysCommand::Policy`].

mod backend;
pub mod bus;

pub use backend::{OutputsRoute, WlrOutputs};

use std::sync::Arc;

use shell_model::volume::Volume;
use tokio::sync::watch;

use crate::env::Env;
use crate::error::ServiceError;
use crate::service::Run;
use shell_model::displays::DisplaysService;
use shell_model::service::ServiceIo;

impl Run for DisplaysService {
    type Kind = Self;
    type Deps = watch::Receiver<Volume>;

    fn run(
        env: &Env,
        deps: Self::Deps,
        io: ServiceIo<Self>,
    ) -> impl Future<Output = Result<(), ServiceError>> + Send + 'static {
        let paths = backend::Paths::of(
            env.buses.system.clone(),
            &env.dirs,
            &env.sys_class,
            Arc::clone(&env.ddc),
            env.wayland.clone(),
        );
        async move {
            backend::run(io.state, io.commands, deps, paths).await;
            Ok(())
        }
    }
}
