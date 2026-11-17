//! The portal's stand-in where there is no portal client (not Linux, or no `desktop`): the
//! same two entry points as `portal/bus.rs`, answering the defaults and never changing.

use super::SystemSettings;
use base_spawn::Spawner;
use settings_store::latest::{self, Receiver};

/// No portal: the caller falls back to the defaults.
#[expect(
    clippy::unused_async,
    reason = "the same signature as portal/bus.rs, whose version awaits the bus"
)]
pub(super) async fn read_all() -> Option<SystemSettings> {
    None
}

/// No portal: the defaults, and nothing ever changes, so the sender drops and the watch stays at
/// its first answer.
#[expect(
    clippy::unused_async,
    reason = "the same signature as portal/bus.rs, whose version awaits the bus"
)]
pub(super) async fn start(_spawn: &dyn Spawner) -> Receiver<SystemSettings> {
    latest::channel(SystemSettings::default()).1
}
