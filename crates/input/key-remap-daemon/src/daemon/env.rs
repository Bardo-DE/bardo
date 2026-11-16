//! What the daemon takes from the process environment, read once in `command::run` so nothing
//! below it reads `std::env`.

use super::discover::{DEV_INPUT, SYS_INPUT, UDEV_DATA, VirtualAllowance};
use super::socket::{SEAT_FILE, seat_user_asked};
use super::source::DEFAULT_SERVICE_SOCKET;
use super::uinput::{PROBE_PERMIT, probe_permitted};
use std::path::PathBuf;
use std::time::Duration;

/// Where the config directory is when nothing overrides it.
const DEFAULT_CONFIG_DIR: &str = key_remap_proto::CONFIG_DIR;
/// Where the vendor config directory is when nothing overrides it.
const DEFAULT_VENDOR_DIR: &str = crate::config::VENDOR_DIR;
/// Where the socket directory is when systemd does not provide one.
const DEFAULT_RUNTIME_DIR: &str = "/run/bardo/key-remap";

/// Where logind records the seat's active user.
const DEFAULT_SEAT_FILE: &str = SEAT_FILE;

/// The process environment as the daemon uses it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Env {
    /// `$NOTIFY_SOCKET`: where `READY=1` goes; none outside systemd. A leading `@` is an
    /// abstract socket.
    pub notify_socket: Option<String>,
    /// `$BARDO_KEY_REMAP_CONFIG_DIR`, else `/etc/bardo/key-remap`.
    pub config_dir: PathBuf,
    /// `$BARDO_KEY_REMAP_VENDOR_DIR`, else `/usr/lib/bardo/key-remap`: where the desktop's shipped defaults are
    /// read from, under the file in the config directory. Injectable so tests need no root.
    pub vendor_dir: PathBuf,
    /// `$RUNTIME_DIRECTORY` (systemd's), else `/run/bardo/key-remap`: where the socket lives.
    pub runtime_dir: PathBuf,
    /// `$BARDO_KEY_REMAP_SEAT_FILE`, else logind's `seat0` state file: where the active user is read
    /// from; injectable so tests need no logind.
    pub seat_file: PathBuf,
    /// `$BARDO_KEY_REMAP_SYS_INPUT`, else `/sys/class/input`: where discovery lists the nodes.
    pub sys_input: PathBuf,
    /// `$BARDO_KEY_REMAP_DEV_INPUT`, else `/dev/input`: where the nodes are opened and watched.
    pub dev_input: PathBuf,
    /// `$BARDO_KEY_REMAP_UDEV_DATA`, else `/run/udev/data`: where discovery reads a node's seat.
    pub udev_data: PathBuf,
    /// `$BARDO_KEY_REMAP_GRAB_VIRTUAL`: the one virtual device, by exact name, discovery may grab. Only
    /// the unit's environment sets it; it exists for tests whose keyboard is a uinput device.
    pub grab_virtual: VirtualAllowance,
    /// `$BARDO_KEY_REMAP_DEVICE_SERVICE`, else `/run/bardo/input-broker/input.sock`: where the service that makes
    /// the virtual devices listens. Devices come from it when the socket is there, and from
    /// `/dev/uinput` when it is not.
    pub device_service: PathBuf,
    /// `$BARDO_KEY_REMAP_STEERING`: who may steer. Only the unit's environment sets it; it exists for
    /// tests whose client is not in the shell's unit.
    pub seat_user: bool,
    /// Whether `--check-uinput` may open the uinput node.
    pub probe: bool,
    /// `$WATCHDOG_USEC` as a duration: how often systemd expects a ping at the latest.
    pub watchdog: Option<Duration>,
}

impl Env {
    /// Reads every variable through `lookup`.
    pub(crate) fn from_lookup(lookup: impl Fn(&str) -> Option<String>) -> Self {
        let non_empty = |k: &str| lookup(k).filter(|s| !s.is_empty());
        let dir =
            |k: &str, default: &str| PathBuf::from(non_empty(k).as_deref().unwrap_or(default));
        Self {
            notify_socket: non_empty("NOTIFY_SOCKET"),
            config_dir: dir("BARDO_KEY_REMAP_CONFIG_DIR", DEFAULT_CONFIG_DIR),
            vendor_dir: dir("BARDO_KEY_REMAP_VENDOR_DIR", DEFAULT_VENDOR_DIR),
            runtime_dir: dir("RUNTIME_DIRECTORY", DEFAULT_RUNTIME_DIR),
            seat_file: dir("BARDO_KEY_REMAP_SEAT_FILE", DEFAULT_SEAT_FILE),
            sys_input: dir("BARDO_KEY_REMAP_SYS_INPUT", SYS_INPUT),
            dev_input: dir("BARDO_KEY_REMAP_DEV_INPUT", DEV_INPUT),
            udev_data: dir("BARDO_KEY_REMAP_UDEV_DATA", UDEV_DATA),
            grab_virtual: VirtualAllowance::from_env_value(
                lookup("BARDO_KEY_REMAP_GRAB_VIRTUAL").as_deref(),
            ),
            device_service: dir("BARDO_KEY_REMAP_DEVICE_SERVICE", DEFAULT_SERVICE_SOCKET),
            seat_user: seat_user_asked(lookup("BARDO_KEY_REMAP_STEERING").as_deref()),
            probe: probe_permitted(lookup(PROBE_PERMIT).as_deref()),
            watchdog: non_empty("WATCHDOG_USEC")
                .and_then(|s| s.parse::<u64>().ok())
                .filter(|&us| us > 0)
                .map(Duration::from_micros),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lookup<'a>(pairs: &'a [(&'a str, &'a str)]) -> impl Fn(&str) -> Option<String> + 'a {
        move |k| {
            pairs
                .iter()
                .find(|(name, _)| *name == k)
                .map(|&(_, v)| v.to_owned())
        }
    }

    #[test]
    fn reads_each_variable_from_the_lookup() {
        let env = Env::from_lookup(lookup(&[
            ("NOTIFY_SOCKET", "/run/systemd/notify"),
            ("BARDO_KEY_REMAP_CONFIG_DIR", "/tmp/cfg"),
            ("BARDO_KEY_REMAP_VENDOR_DIR", "/tmp/vendor"),
            ("RUNTIME_DIRECTORY", "/run/bardo/key-remap"),
            (PROBE_PERMIT, "1"),
            ("WATCHDOG_USEC", "1000000"),
            ("BARDO_KEY_REMAP_SEAT_FILE", "/tmp/seat0"),
            ("BARDO_KEY_REMAP_SYS_INPUT", "/tmp/sys"),
            ("BARDO_KEY_REMAP_DEV_INPUT", "/tmp/dev"),
            ("BARDO_KEY_REMAP_UDEV_DATA", "/tmp/udev"),
            ("BARDO_KEY_REMAP_GRAB_VIRTUAL", "test keyboard"),
            ("BARDO_KEY_REMAP_DEVICE_SERVICE", "/tmp/input.sock"),
            ("BARDO_KEY_REMAP_STEERING", "seat-user"),
        ]));
        assert_eq!(env.notify_socket.as_deref(), Some("/run/systemd/notify"));
        assert_eq!(env.config_dir, PathBuf::from("/tmp/cfg"));
        assert_eq!(env.vendor_dir, PathBuf::from("/tmp/vendor"));
        assert_eq!(env.runtime_dir, PathBuf::from("/run/bardo/key-remap"));
        assert_eq!(env.seat_file, PathBuf::from("/tmp/seat0"));
        assert_eq!(env.sys_input, PathBuf::from("/tmp/sys"));
        assert_eq!(env.dev_input, PathBuf::from("/tmp/dev"));
        assert_eq!(env.udev_data, PathBuf::from("/tmp/udev"));
        assert_eq!(
            env.grab_virtual,
            VirtualAllowance::Named("test keyboard".to_owned())
        );
        assert_eq!(env.device_service, PathBuf::from("/tmp/input.sock"));
        assert!(env.seat_user);
        assert!(env.probe);
        assert_eq!(env.watchdog, Some(Duration::from_secs(1)));
    }

    #[test]
    fn an_empty_environment_has_the_defaults() {
        let env = Env::from_lookup(lookup(&[]));
        assert_eq!(env.notify_socket, None);
        assert_eq!(env.config_dir, PathBuf::from("/etc/bardo/key-remap"));
        assert_eq!(env.vendor_dir, PathBuf::from("/usr/lib/bardo/key-remap"));
        assert_eq!(env.runtime_dir, PathBuf::from("/run/bardo/key-remap"));
        assert_eq!(env.seat_file, PathBuf::from("/run/systemd/seats/seat0"));
        assert_eq!(env.sys_input, PathBuf::from("/sys/class/input"));
        assert_eq!(env.dev_input, PathBuf::from("/dev/input"));
        assert_eq!(env.udev_data, PathBuf::from("/run/udev/data"));
        assert_eq!(env.grab_virtual, VirtualAllowance::Nobody);
        assert_eq!(
            env.device_service,
            PathBuf::from("/run/bardo/input-broker/input.sock")
        );
        assert!(!env.seat_user);
        assert!(!env.probe);
        assert_eq!(env.watchdog, None);
    }

    #[test]
    fn systemds_runtime_directory_wins_and_odd_values_fall_back() {
        type Case = (
            &'static str,
            &'static [(&'static str, &'static str)],
            &'static str,
            Option<Duration>,
        );
        const CASES: &[Case] = &[
            (
                "runtime directory from systemd",
                &[("RUNTIME_DIRECTORY", "/run/custom")],
                "/run/custom",
                None,
            ),
            (
                "empty runtime directory",
                &[("RUNTIME_DIRECTORY", "")],
                "/run/bardo/key-remap",
                None,
            ),
            (
                "unparsable watchdog",
                &[("WATCHDOG_USEC", "soon")],
                "/run/bardo/key-remap",
                None,
            ),
            (
                "zero watchdog",
                &[("WATCHDOG_USEC", "0")],
                "/run/bardo/key-remap",
                None,
            ),
            (
                "500 ms watchdog",
                &[("WATCHDOG_USEC", "500000")],
                "/run/bardo/key-remap",
                Some(Duration::from_millis(500)),
            ),
        ];
        for &(name, vars, runtime, watchdog) in CASES {
            let env = Env::from_lookup(lookup(vars));
            assert_eq!(env.runtime_dir, PathBuf::from(runtime), "{name}");
            assert_eq!(env.watchdog, watchdog, "{name}");
        }
    }
}
