//! The accountd binary started on a private bus in a [`TestEnv`], and the wait for it to serve.

use std::path::PathBuf;
use std::process::Stdio;

use base_testkit::{PrivateBus, Reaped, TestEnv};

/// The daemon process, killed by its PID when the guard goes.
pub struct Daemon {
    pub child: Reaped,
    stderr: PathBuf,
}

impl Daemon {
    /// Everything the daemon wrote to its standard error so far.
    pub fn stderr(&self) -> String {
        std::fs::read_to_string(&self.stderr).unwrap_or_default()
    }

    /// Waits for the process to end.
    pub fn wait(&mut self) -> std::process::ExitStatus {
        self.child.child_mut().wait().expect("exits")
    }
}

/// A scratch environment on `bus` that keeps accountd off this computer's Tailscale and `/etc/bardo`.
pub fn env_on(bus: &PrivateBus) -> TestEnv {
    let env = TestEnv::new("accountd").with_bus(bus);
    let root = env.root().to_owned();
    // A socket in the scratch directory that nobody serves; a system caller table that is absent,
    // so empty: nobody has a fixed role.
    env.with(
        account_daemon::paths::TAILSCALE_SOCKET_VAR,
        &root.join("no-tailscale.sock").display().to_string(),
    )
    .with(
        account_callers::SYSTEM_CALLERS_VAR,
        &root.join("system-callers.toml").display().to_string(),
    )
}

/// Starts the real `bardo-accountd` in `env` with `args`.
pub fn spawn(env: &TestEnv, args: &[&std::ffi::OsStr]) -> Daemon {
    let stderr = env.root().join("stderr.log");
    let mut command = env.command(env!("CARGO_BIN_EXE_bardo-accountd"));
    command
        .args(args)
        .stdout(Stdio::null())
        .stderr(std::fs::File::create(&stderr).expect("stderr file"));
    Daemon {
        child: Reaped::spawn(&mut command).expect("accountd starts"),
        stderr,
    }
}

/// Whether the daemon came to own its bus name (false: it ended first).
pub async fn serving(bus: &PrivateBus, daemon: &mut Daemon) -> bool {
    let probe = bus.connect().await;
    let dbus = zbus::fdo::DBusProxy::new(&probe).await.expect("proxy");
    // The daemon is a process that has to be started and run; on a starved machine that takes
    // many seconds, not the five an idle one needs, and it gets as long as the helper's deadline.
    base_testkit::eventually_some_async("accountd to own its name or end", || {
        let exited = daemon.child.child_mut().try_wait().ok().flatten().is_some();
        let dbus = dbus.clone();
        async move {
            if dbus
                .name_has_owner(account_wire::names::ACCOUNTS_BUS.try_into().expect("name"))
                .await
                .unwrap_or(false)
            {
                return Some(true);
            }
            exited.then_some(false)
        }
    })
    .await
}
