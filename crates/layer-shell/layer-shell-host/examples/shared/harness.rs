//! The host configuration the acceptance scripts run the demos with.
//!
//! `scripts/dev/lib/compositor.sh` sets `BARDO_LAYER_SHELL_HOST_BACKEND=cpu` for a headless compositor with
//! no GPU behind it (the compositor in its jail): the demos then paint into `wl_shm` buffers
//! (`Backend::Cpu`) instead of probing for a GPU adapter. Unset (`KWin`, a developer's shell) or
//! `gpu`, the demos run the library default, so the F80 adapter fallback stays under test.

use layer_shell_host::{Backend, HostConfig};

/// The environment variable that names the backend.
const BACKEND_VAR: &str = "BARDO_LAYER_SHELL_HOST_BACKEND";

/// The library default, with the backend `BARDO_LAYER_SHELL_HOST_BACKEND` names (`cpu` or `gpu`).
pub fn host_config() -> HostConfig {
    HostConfig {
        backend: backend(std::env::var(BACKEND_VAR).ok().as_deref()),
        ..HostConfig::default()
    }
}

/// `cpu` is [`Backend::Cpu`]; anything else, or nothing, is the default.
fn backend(name: Option<&str>) -> Backend {
    match name {
        Some("cpu") => Backend::Cpu,
        _ => Backend::default(),
    }
}
