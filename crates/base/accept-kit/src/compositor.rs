//! A headless compositor and the fake logind it needs, on private buses, for a test that talks
//! Wayland. It ports `start_compositor` of `crates/compositor/dev/lib/compositor.sh`.
//!
//! The tools are prebuilt binaries ([`crate::bin`]): `bardo-compositor` and `fake-logind`.

use std::fs::File;
use std::io;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::time::Duration;

use base_testkit::{Deadline, PrivateBus, Reaped, TestEnv};

/// Which clock the compositor runs on.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Clock {
    /// Virtual time that only a test advances (`bardo-compositorctl debug clock`).
    #[default]
    Stepped,
    /// The wall clock.
    Wall,
}

/// What a [`Compositor`] is started with.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Options {
    /// The clock; stepped unless asked otherwise.
    pub clock: Clock,
    /// `--outputs`, e.g. `1920x1080`; the compositor's default (one output) when `None`.
    pub outputs: Option<String>,
    /// Further arguments, passed through as they are.
    pub args: Vec<String>,
}

impl Options {
    /// The compositor's command line, after the program name. `config_home` is where it keeps its
    /// settings.
    #[must_use]
    pub fn args(&self, config_home: &Path) -> Vec<String> {
        let clock = match self.clock {
            Clock::Stepped => "stepped",
            Clock::Wall => "wall",
        };
        let mut args: Vec<String> = [
            "--backend",
            "headless",
            "--clock",
            clock,
            "--dmabuf",
            "none",
            "--config-home",
        ]
        .map(str::to_owned)
        .into();
        args.push(config_home.display().to_string());
        if let Some(outputs) = &self.outputs {
            args.extend(["--outputs".to_owned(), outputs.clone()]);
        }
        args.extend(self.args.iter().cloned());
        args
    }
}

/// A running compositor with its buses and fake logind; all of it ends when this drops.
#[derive(Debug)]
pub struct Compositor {
    // Declared in the order they must go: the compositor, then logind, then the buses.
    server: Reaped,
    logind: Reaped,
    session: PrivateBus,
    system: PrivateBus,
    socket: PathBuf,
}

impl Compositor {
    /// Starts the system bus and the session bus, `fake-logind` on the system bus, then
    /// `bardo-compositor --backend headless`, and waits for its Wayland socket in `env`'s runtime
    /// directory. Logs go to `<root>/logs`.
    ///
    /// # Errors
    /// An I/O error of starting a process, or the compositor ending (the tail of its log is in the
    /// message) or showing no socket within [`base_testkit::GENEROUS`].
    ///
    /// # Panics
    /// When a prebuilt binary is missing ([`crate::bin`]) or `dbus-daemon` is.
    pub fn start(env: &TestEnv, options: &Options) -> io::Result<Self> {
        let session = PrivateBus::start_in(&env.dir("bus-session"));
        let system = PrivateBus::start_system_in(&env.dir("bus-system"));
        let buses = [
            ("DBUS_SESSION_BUS_ADDRESS", session.address().to_owned()),
            ("DBUS_SYSTEM_BUS_ADDRESS", system.address().to_owned()),
        ];
        let logs = env.dir("logs");
        let log = |name: &str| File::create(logs.join(name));
        let logind = Reaped::spawn(
            env.command(crate::bin("fake-logind"))
                .envs(buses.iter().map(|(k, v)| (*k, v)))
                .arg(system.address())
                .stdin(Stdio::null())
                .stdout(log("fake-logind.log")?)
                .stderr(log("fake-logind.err")?),
        )?;
        let before = sockets(env.runtime());
        let mut compositor = Reaped::spawn(
            env.command(crate::bin("bardo-compositor"))
                .envs(buses.iter().map(|(k, v)| (*k, v)))
                .args(options.args(&env.config()))
                .stdin(Stdio::null())
                .stdout(log("bardo-compositor.log")?)
                .stderr(log("bardo-compositor.err")?),
        )?;
        let deadline = Deadline::generous();
        let socket = loop {
            if let Some(status) = compositor.child_mut().try_wait()? {
                return Err(io::Error::other(format!(
                    "bardo-compositor exited ({status}) before its socket appeared; {}",
                    tail(&logs.join("bardo-compositor.err"))
                )));
            }
            if let Some(socket) = socket_added(&before, &sockets(env.runtime())) {
                break socket;
            }
            if deadline.passed() {
                return Err(io::Error::other(format!(
                    "no wayland socket after {:?}; {}",
                    deadline.waited(),
                    tail(&logs.join("bardo-compositor.err"))
                )));
            }
            std::thread::sleep(Duration::from_millis(100));
        };
        Ok(Self {
            server: compositor,
            logind,
            session,
            system,
            socket,
        })
    }

    /// The compositor's socket (an absolute path, for `WAYLAND_DISPLAY`).
    #[must_use]
    pub fn socket(&self) -> &Path {
        &self.socket
    }

    /// The compositor's PID.
    #[must_use]
    pub fn pid(&self) -> u32 {
        self.server.pid()
    }

    /// The variables a client needs besides the environment's own: `WAYLAND_DISPLAY` and both bus
    /// addresses.
    #[must_use]
    pub fn client_vars(&self) -> Vec<(String, String)> {
        vec![
            (
                "WAYLAND_DISPLAY".to_owned(),
                self.socket.display().to_string(),
            ),
            (
                "DBUS_SESSION_BUS_ADDRESS".to_owned(),
                self.session.address().to_owned(),
            ),
            (
                "DBUS_SYSTEM_BUS_ADDRESS".to_owned(),
                self.system.address().to_owned(),
            ),
        ]
    }

    /// The fake logind's PID.
    #[must_use]
    pub fn logind_pid(&self) -> u32 {
        self.logind.pid()
    }
}

/// The `wayland-*` sockets (not their `.lock` files) in `dir`, sorted.
fn sockets(dir: &Path) -> Vec<PathBuf> {
    let mut found: Vec<PathBuf> = std::fs::read_dir(dir)
        .into_iter()
        .flatten()
        .filter_map(|entry| entry.ok().map(|e| e.path()))
        .filter(|path| is_socket_name(path))
        .collect();
    found.sort();
    found
}

/// Whether `path` is named like a compositor's socket: `wayland-<n>`, not `wayland-<n>.lock`.
fn is_socket_name(path: &Path) -> bool {
    path.file_name()
        .and_then(|n| n.to_str())
        .is_some_and(|n| n.starts_with("wayland-"))
        && !path
            .extension()
            .is_some_and(|e| e.eq_ignore_ascii_case("lock"))
}

/// The first socket in `now` that was not in `before`.
fn socket_added(before: &[PathBuf], now: &[PathBuf]) -> Option<PathBuf> {
    now.iter().find(|p| !before.contains(p)).cloned()
}

/// The last lines of a log, for an error message.
fn tail(path: &Path) -> String {
    let text = std::fs::read_to_string(path).unwrap_or_default();
    let lines: Vec<&str> = text.lines().collect();
    let from = lines.len().saturating_sub(20);
    format!("{}:\n{}", path.display(), lines[from..].join("\n"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_defaults_are_headless_stepped_no_dmabuf() {
        assert_eq!(
            Options::default().args(Path::new("/c")),
            [
                "--backend",
                "headless",
                "--clock",
                "stepped",
                "--dmabuf",
                "none",
                "--config-home",
                "/c"
            ]
        );
    }

    #[test]
    fn outputs_and_extra_arguments_follow() {
        let options = Options {
            clock: Clock::Wall,
            outputs: Some("800x600".into()),
            args: vec!["--xwayland".into(), "off".into()],
        };
        assert_eq!(
            options.args(Path::new("/c")),
            [
                "--backend",
                "headless",
                "--clock",
                "wall",
                "--dmabuf",
                "none",
                "--config-home",
                "/c",
                "--outputs",
                "800x600",
                "--xwayland",
                "off"
            ]
        );
    }

    #[test]
    fn a_socket_is_named_wayland_n_and_a_lock_is_not() {
        assert!(is_socket_name(Path::new("/r/wayland-0")));
        assert!(!is_socket_name(Path::new("/r/wayland-0.lock")));
        assert!(!is_socket_name(Path::new("/r/bus")));
    }

    #[test]
    fn the_new_socket_is_the_one_not_there_before() {
        let before = vec![PathBuf::from("/r/wayland-0")];
        let now = vec![PathBuf::from("/r/wayland-0"), PathBuf::from("/r/wayland-1")];
        assert_eq!(socket_added(&before, &before), None);
        assert_eq!(
            socket_added(&before, &now),
            Some(PathBuf::from("/r/wayland-1"))
        );
    }
}
