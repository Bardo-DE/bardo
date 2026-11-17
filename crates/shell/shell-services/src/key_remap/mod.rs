//! The key remap daemon, the Mac keyboard daemon, as bardo-shell talks to it.
//!
//! bardo-shell tells it which profile the focused window wants and the person's `key-remap.toml`,
//! learns from it whether it runs and holds the keyboards, and tells the person when that needs
//! them (`shell_model::key_remap::attention`).
//!
//! The socket (`/run/bardo/key-remap/key-remap.sock`, `Env::key_remap_socket`) is held by a thread of its own,
//! started by `run`: it reconnects with a growing wait and starts each connection with the focus
//! and the config. While the socket is down for more than a few seconds the service asks systemd
//! about the unit on the system bus, so a crash loop that gave up shows as `Failed`; a machine
//! with no key remap daemon shows as `NotInstalled`. `Retry` resets and restarts the unit, `Resume` ends the
//! pause the escape chord caused.
//!
//! Nothing is sent without a socket path (`Env::isolated` has none) and no unit is asked about
//! without a system bus, so a test or a script never touches the real daemon.
//!
//! `Overrides` (the person's `keyboard.overrides`) replaces the service's own and re-chooses the
//! focused window's profile, sending `Focus` when it changed.
//!
//! The notices carry no action, so recovery is `bardo-shell key_remap retry` and `bardo-shell key_remap resume`.

mod backend;

use crate::env::Env;
use crate::error::ServiceError;
use crate::service::Run;
use shell_model::key_remap::KeyRemapService;
use shell_model::service::ServiceIo;

pub use backend::KeyRemapLinks;

impl Run for KeyRemapService {
    type Kind = Self;
    type Deps = KeyRemapLinks;

    fn run(
        env: &Env,
        deps: Self::Deps,
        io: ServiceIo<Self>,
    ) -> impl Future<Output = Result<(), ServiceError>> + Send + 'static {
        let reach = backend::Reach::of(env);
        async move { backend::run(reach, deps, io).await }
    }
}
