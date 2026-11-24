//! A private session bus for one test: a `dbus-daemon` made from a scratch config in a scratch
//! directory, with nothing from the environment. The person's real bus is never named.
//!
//! Teardown is certain. The daemon runs with `--nofork`, so the process this crate spawns IS the
//! daemon and its PID is the daemon's (never a shell's). `PrivateBus` owns it through a
//! [`Reaped`] guard that is made before anything else can fail, so the daemon is killed by PID and
//! waited for when the bus drops: at the end of the test, on a panic, on an early return. And if
//! the test process itself dies without unwinding (a timeout's SIGKILL), the guard's watchdog
//! kills the daemon by PID once it sees the test process gone. No kill here goes by a name or a
//! pattern.
//!
//! Without `dbus-daemon` there is no bus to test against, and a test never falls back to the real
//! session bus: [`PrivateBus`]'s constructors panic, saying how to install it, so a missing tool
//! is a failing test and never a silent pass.

use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use crate::{Reaped, Scratch};

const CONFIG: &str = r#"<!DOCTYPE busconfig PUBLIC "-//freedesktop//DTD D-Bus Bus Configuration 1.0//EN"
 "http://www.freedesktop.org/standards/dbus/1.0/busconfig.dtd">
<busconfig>
  <type>session</type>
  <listen>unix:path=SOCKET</listen>
  <auth>EXTERNAL</auth>
  SERVICEDIR
  <policy context="default">
    RULES
  </policy>
</busconfig>
"#;

/// What the bus's default policy lets a connection do beyond sending, receiving and owning.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Policy {
    Plain,
    Eavesdrop,
}

impl Policy {
    fn rules(self) -> &'static str {
        match self {
            Self::Plain => {
                r#"<allow send_destination="*"/>
    <allow receive_sender="*"/>
    <allow own="*"/>"#
            }
            Self::Eavesdrop => {
                r#"<allow send_destination="*" eavesdrop="true"/>
    <allow eavesdrop="true"/>
    <allow own="*"/>"#
            }
        }
    }
}

/// A bus of our own, killed when the test ends.
#[derive(Debug)]
pub struct PrivateBus {
    // Declared first: the daemon is killed and waited for before anything else of ours goes.
    daemon: Reaped,
    address: String,
    dir: PathBuf,
    _owned: Option<Scratch>,
}

impl PrivateBus {
    /// A bus in a scratch directory of its own, removed with the bus.
    ///
    /// # Panics
    ///
    /// When `dbus-daemon` is missing or does not print its address.
    #[must_use]
    pub fn start() -> Self {
        let scratch = Scratch::new("bus");
        let dir = scratch.path().to_owned();
        Self::launch(dir, Policy::Plain, None, &[], Some(scratch))
    }

    /// A bus in `dir`, which the caller owns.
    ///
    /// # Panics
    ///
    /// When `dbus-daemon` is missing or does not print its address.
    #[must_use]
    pub fn start_in(dir: &Path) -> Self {
        Self::launch(dir.to_owned(), Policy::Plain, None, &[], None)
    }

    /// A bus in `dir` that also activates the services described in `servicedir`, and whose own
    /// environment (what an activated service inherits) is `env`.
    ///
    /// # Panics
    ///
    /// When `dbus-daemon` is missing or does not print its address.
    #[must_use]
    pub fn start_with(dir: &Path, servicedir: Option<&Path>, env: &[(&str, &str)]) -> Self {
        Self::launch(dir.to_owned(), Policy::Plain, servicedir, env, None)
    }

    /// A bus in `dir` whose connections may eavesdrop (for a test that watches what a binary
    /// sends).
    ///
    /// # Panics
    ///
    /// When `dbus-daemon` is missing or does not print its address.
    #[must_use]
    pub fn start_eavesdropping(dir: &Path) -> Self {
        Self::launch(dir.to_owned(), Policy::Eavesdrop, None, &[], None)
    }

    /// A bus in a scratch directory of its own that can start the services `names`: each has a
    /// service file whose program only writes down that it was started ([`Self::started`]), then
    /// exits without taking the name. A name not listed has no service file, so asking the bus to
    /// start it fails. The scratch directory is also HOME and the runtime directory of what the
    /// bus starts.
    ///
    /// # Panics
    ///
    /// When `dbus-daemon` is missing or does not print its address.
    #[must_use]
    pub fn start_activatable(names: &[&str]) -> Self {
        let scratch = Scratch::new("bus");
        let dir = scratch.path().to_owned();
        let services = dir.join("services");
        std::fs::create_dir_all(&services).expect("the service directory");
        for name in names {
            let exec = format!(
                "/usr/bin/touch {}",
                dir.join(format!("started-{name}")).display()
            );
            std::fs::write(
                services.join(format!("{name}.service")),
                format!("[D-BUS Service]\nName={name}\nExec={exec}\n"),
            )
            .expect("a service file");
        }
        let home = dir.to_string_lossy().into_owned();
        Self::launch(
            dir,
            Policy::Plain,
            Some(&services),
            &[("HOME", &home), ("XDG_RUNTIME_DIR", &home)],
            Some(scratch),
        )
    }

    fn launch(
        dir: PathBuf,
        policy: Policy,
        servicedir: Option<&Path>,
        env: &[(&str, &str)],
        owned: Option<Scratch>,
    ) -> Self {
        let socket = dir.join("bus.sock");
        let config = dir.join("bus.conf");
        let services = servicedir
            .map(|d| format!("<servicedir>{}</servicedir>", d.display()))
            .unwrap_or_default();
        let text = CONFIG
            .replace("RULES", policy.rules())
            .replace("SOCKET", &socket.to_string_lossy())
            .replace("SERVICEDIR", &services);
        std::fs::write(&config, text).expect("bus config");
        let mut daemon = match Reaped::spawn(
            Command::new("dbus-daemon")
                .arg(format!("--config-file={}", config.display()))
                .args(["--nofork", "--print-address=1"])
                .env_clear()
                .envs(env.iter().copied())
                .stdout(Stdio::piped())
                .stderr(Stdio::null()),
        ) {
            Ok(daemon) => daemon,
            Err(error) => panic!(
                "dbus-daemon cannot be started ({error}); it is a required test tool, install it (Fedora: dnf install dbus-daemon; Debian/Ubuntu: apt install dbus-daemon)"
            ),
        };
        // From here the guard owns the daemon: a failure below unwinds through its `Drop`.
        let stdout = daemon.child_mut().stdout.take().expect("stdout");
        let mut address = String::new();
        BufReader::new(stdout)
            .read_line(&mut address)
            .expect("the daemon prints its address");
        assert!(
            address.starts_with("unix:"),
            "the daemon printed no address: {address:?}"
        );
        Self {
            daemon,
            address: address.trim().to_owned(),
            dir,
            _owned: owned,
        }
    }

    /// The address a daemon started by the test is told to use.
    #[must_use]
    pub fn address(&self) -> &str {
        &self.address
    }

    /// The directory the bus lives in: its socket and config, and for [`Self::start`] and
    /// [`Self::start_activatable`] a scratch directory the test may use too.
    #[must_use]
    pub fn scratch(&self) -> &Path {
        &self.dir
    }

    /// The PID of the `dbus-daemon` itself.
    #[must_use]
    pub fn pid(&self) -> u32 {
        self.daemon.pid()
    }

    /// The services this bus started, sorted (those [`Self::start_activatable`] gave a service
    /// file only).
    ///
    /// # Panics
    ///
    /// When the bus directory cannot be read.
    #[must_use]
    pub fn started(&self) -> Vec<String> {
        let mut names: Vec<String> = std::fs::read_dir(&self.dir)
            .expect("the bus directory")
            .filter_map(|entry| entry.ok()?.file_name().to_str().map(str::to_owned))
            .filter_map(|file| file.strip_prefix("started-").map(str::to_owned))
            .collect();
        names.sort();
        names
    }

    /// A connection builder aimed at this bus and no other.
    ///
    /// # Panics
    ///
    /// When the bus address is not valid.
    #[cfg(feature = "dbus")]
    #[must_use]
    pub fn builder(&self) -> zbus::connection::Builder<'_> {
        zbus::connection::Builder::address(self.address.as_str()).expect("a bus address")
    }

    /// A new connection to this bus.
    ///
    /// # Panics
    ///
    /// When the connection cannot be made: a test has no bus to fall back to.
    #[cfg(feature = "dbus")]
    pub async fn connect(&self) -> zbus::Connection {
        self.builder()
            .build()
            .await
            .expect("connect to the private bus")
    }
}

/// The unique bus name of `connection`.
///
/// # Panics
///
/// When `connection` has no unique name.
#[cfg(feature = "dbus")]
#[must_use]
pub fn name_of(connection: &zbus::Connection) -> String {
    connection
        .unique_name()
        .map(|name| name.as_str().to_owned())
        .expect("a unique name")
}
