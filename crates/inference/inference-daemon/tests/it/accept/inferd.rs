//! The environment the real `bardo-inferd` binary is started in.

use base_testkit::{PrivateBus, TestEnv};

/// A scratch environment on `bus`: no system caller table (absent, so empty: nobody has a fixed
/// role) in place of this computer's `/etc/bardo`, and a Tailscale nobody serves.
pub fn env_on(bus: &PrivateBus) -> TestEnv {
    let env = TestEnv::new("inferd").with_bus(bus);
    let root = env.root().to_owned();
    let at = |name: &str| root.join(name).display().to_string();
    env.with(
        account_callers::INFERD_CALLERS.var,
        &at("inferd-callers.toml"),
    )
    .with("BARDO_INFERD_TAILSCALE_SOCKET", &at("no-tailscale.sock"))
}
