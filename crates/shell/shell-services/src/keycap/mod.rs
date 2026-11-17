//! keycap, the Mac keyboard daemon, as bardo-shell talks to it.
//!
//! bardo-shell tells it which profile the focused window wants and the person's `key-remap.toml`,
//! learns from it whether it runs and holds the keyboards, and tells the person when that needs
//! them (`shell_model::keycap::attention`).
//!
//! The socket (`/run/bardo/key-remap/key-remap.sock`, `Env::keycap_socket`) is held by a thread of its own,
//! started by `run`: it reconnects with a growing wait and starts each connection with the focus
//! and the config. While the socket is down for more than a few seconds the service asks systemd
//! about the unit on the system bus, so a crash loop that gave up shows as `Failed`; a machine
//! with no keycap shows as `NotInstalled`. `Retry` resets and restarts the unit, `Resume` ends the
//! pause the escape chord caused.
//!
//! Nothing is sent without a socket path (`Env::isolated` has none) and no unit is asked about
//! without a system bus, so a test or a script never touches the real daemon.
//!
//! `Overrides` (the person's `keyboard.overrides`) replaces the service's own and re-chooses the
//! focused window's profile, sending `Focus` when it changed.
//!
//! The notices carry no action, so recovery is `bardo-shell keycap retry` and `bardo-shell keycap resume`.

mod backend;

use crate::env::Env;
use crate::error::ServiceError;
use crate::service::Run;
use shell_model::keycap::KeycapService;
use shell_model::service::ServiceIo;

pub use backend::KeycapLinks;

impl Run for KeycapService {
    type Deps = KeycapLinks;

    fn run(
        env: &Env,
        deps: Self::Deps,
        io: ServiceIo<Self>,
    ) -> impl Future<Output = Result<(), ServiceError>> + Send + 'static {
        let reach = backend::Reach::of(env);
        async move { backend::run(reach, deps, io).await }
    }
}
