//! The environment a test gives a process it starts: a scratch HOME and XDG tree, a private
//! runtime directory, and buses that point at nothing unless the test names one.

use std::ffi::OsStr;
use std::io;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::{PrivateBus, Reaped, Scratch};

/// A Unix socket's path is at most 107 bytes; a runtime directory whose own path is longer than
/// this gets a short link, so the sockets a daemon makes under it (`<run>/bardo/x/y-<id>.sock`)
/// still fit.
const MAX_RUN_PATH: usize = 40;

/// Where the short link to a long runtime directory is made (`/tmp` is the one place every
/// machine has and that is short).
const LINK_DIR: &str = "/tmp";

/// A scratch root with `home`, `config`, `data`, `state`, `cache` and `run` under it, and the
/// environment that makes a process use them and nothing of the person's.
///
/// This is the list `xtask/test-env.sh` gives every test process (a unit test keeps the two in
/// agreement), made once more for a process a test starts itself: [`Self::command`] clears the
/// environment and sets [`Self::vars`]; [`Self::spawn`] does that and guards the child with
/// [`Reaped`].
///
/// The root is removed when this drops; drop whatever runs inside it first.
#[derive(Debug)]
pub struct TestEnv {
    // Declared first: the link goes (in `Drop`) before the root is removed.
    link: Option<PathBuf>,
    root: Scratch,
    run: PathBuf,
    extra: Vec<(String, String)>,
}

impl TestEnv {
    /// A fresh root labelled `label`; its `run` directory is private (0700).
    ///
    /// # Panics
    /// When a directory cannot be made.
    #[must_use]
    pub fn new(label: &str) -> Self {
        Self::make(label, 0o700, 0o700)
    }

    /// A fresh root whose directories are 0755, for a test that hands them to another user's
    /// process (a device test running a helper that drops privileges).
    ///
    /// # Panics
    /// When a directory cannot be made.
    #[must_use]
    pub fn new_open(label: &str) -> Self {
        Self::make(label, 0o755, 0o755)
    }

    fn make(label: &str, dirs: u32, run: u32) -> Self {
        let root = Scratch::new(label);
        for name in SUBDIRS {
            let mode = if *name == "run" { run } else { dirs };
            let path = root.join(name);
            std::fs::create_dir_all(&path).expect("a test env directory");
            std::fs::set_permissions(&path, std::fs::Permissions::from_mode(mode))
                .expect("a test env directory's mode");
        }
        let real_run = root.join("run");
        let (run, link) = if real_run.as_os_str().len() > MAX_RUN_PATH {
            let link = short_run(&root);
            let _ = std::fs::remove_file(&link);
            std::os::unix::fs::symlink(&real_run, &link).expect("the short runtime link");
            (link.clone(), Some(link))
        } else {
            (real_run, None)
        };
        Self {
            link,
            root,
            run,
            extra: Vec::new(),
        }
    }

    /// The root every other directory is under.
    #[must_use]
    pub fn root(&self) -> &Path {
        self.root.path()
    }

    /// `HOME`.
    #[must_use]
    pub fn home(&self) -> PathBuf {
        self.root.join("home")
    }

    /// `XDG_CONFIG_HOME`.
    #[must_use]
    pub fn config(&self) -> PathBuf {
        self.root.join("config")
    }

    /// `XDG_DATA_HOME`.
    #[must_use]
    pub fn data(&self) -> PathBuf {
        self.root.join("data")
    }

    /// `XDG_STATE_HOME`.
    #[must_use]
    pub fn state(&self) -> PathBuf {
        self.root.join("state")
    }

    /// `XDG_CACHE_HOME`.
    #[must_use]
    pub fn cache(&self) -> PathBuf {
        self.root.join("cache")
    }

    /// `XDG_RUNTIME_DIR`: `<root>/run`, or a short link to it when the root's path is too long for
    /// a socket under it.
    #[must_use]
    pub fn runtime(&self) -> &Path {
        &self.run
    }

    /// A new directory `name` under the root (made if missing), for whatever else a test needs.
    ///
    /// # Panics
    /// When the directory cannot be made.
    #[must_use]
    pub fn dir(&self, name: &str) -> PathBuf {
        let path = self.root.join(name);
        std::fs::create_dir_all(&path).expect("a test env directory");
        path
    }

    /// Sets one more variable (or replaces one of [`Self::vars`]) for processes this starts.
    #[must_use]
    pub fn with(mut self, key: &str, value: &str) -> Self {
        set(&mut self.extra, key, value);
        self
    }

    /// Points `DBUS_SESSION_BUS_ADDRESS` at `bus`.
    #[must_use]
    pub fn with_bus(self, bus: &PrivateBus) -> Self {
        self.with("DBUS_SESSION_BUS_ADDRESS", bus.address())
    }

    /// Points `DBUS_SYSTEM_BUS_ADDRESS` at `bus` (see [`PrivateBus::start_system`]).
    #[must_use]
    pub fn with_system_bus(self, bus: &PrivateBus) -> Self {
        self.with("DBUS_SYSTEM_BUS_ADDRESS", bus.address())
    }

    /// Every variable a process of this environment gets, and nothing else.
    #[must_use]
    pub fn vars(&self) -> Vec<(String, String)> {
        let mut vars = env_list(self.root.path(), &self.run);
        for (key, value) in &self.extra {
            set(&mut vars, key, value);
        }
        vars
    }

    /// `exe` with an empty environment but [`Self::vars`], run from the root.
    #[must_use]
    pub fn command(&self, exe: impl AsRef<OsStr>) -> Command {
        let mut command = Command::new(exe);
        command
            .env_clear()
            .envs(self.vars())
            .current_dir(self.root.path());
        command
    }

    /// Starts `exe args` in this environment, guarded by [`Reaped`] (killed by PID when the guard
    /// drops, and by the watchdog if the test dies).
    ///
    /// # Errors
    /// The I/O error of starting the process.
    pub fn spawn<I, S>(&self, exe: impl AsRef<OsStr>, args: I) -> io::Result<Reaped>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>,
    {
        Reaped::spawn(self.command(exe).args(args))
    }
}

impl Drop for TestEnv {
    fn drop(&mut self) {
        if let Some(link) = &self.link
            && link
                .symlink_metadata()
                .is_ok_and(|m| m.file_type().is_symlink())
        {
            let _ = std::fs::remove_file(link);
        }
    }
}

/// The directories of a root, in the order they are made.
const SUBDIRS: &[&str] = &["home", "config", "data", "state", "cache", "run"];

fn set(vars: &mut Vec<(String, String)>, key: &str, value: &str) {
    match vars.iter_mut().find(|(k, _)| k == key) {
        Some(slot) => value.clone_into(&mut slot.1),
        None => vars.push((key.to_owned(), value.to_owned())),
    }
}

/// The variables of an environment rooted at `root` whose runtime directory is `run`.
fn env_list(root: &Path, run: &Path) -> Vec<(String, String)> {
    let at = |p: &str| root.join(p).display().to_string();
    let none = at("none");
    [
        ("HOME", at("home")),
        ("XDG_CONFIG_HOME", at("config")),
        ("XDG_DATA_HOME", at("data")),
        ("XDG_STATE_HOME", at("state")),
        ("XDG_CACHE_HOME", at("cache")),
        ("XDG_DATA_DIRS", none.clone()),
        ("XDG_CONFIG_DIRS", none),
        ("XDG_RUNTIME_DIR", run.display().to_string()),
        ("TMPDIR", root.display().to_string()),
        // What systemd gives a user unit: a daemon's compilers find `ld` on it.
        ("PATH", "/usr/bin:/bin".to_owned()),
        (
            "DBUS_SESSION_BUS_ADDRESS",
            format!("unix:path={}/no-session-bus", run.display()),
        ),
        (
            "DBUS_SYSTEM_BUS_ADDRESS",
            format!("unix:path={}/no-system-bus", run.display()),
        ),
    ]
    .into_iter()
    .map(|(k, v)| (k.to_owned(), v))
    .collect()
}

/// The link's path: `/tmp/dl-<16 hex digits of the root's FNV-1a hash>`.
fn short_run(root: &Path) -> PathBuf {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for byte in root.as_os_str().as_encoded_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x0100_0000_01b3);
    }
    Path::new(LINK_DIR).join(format!("dl-{hash:016x}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn get<'a>(vars: &'a [(String, String)], key: &str) -> Option<&'a str> {
        vars.iter().find(|(k, _)| k == key).map(|(_, v)| v.as_str())
    }

    #[test]
    fn the_list_points_everything_into_the_root() {
        let vars = env_list(Path::new("/r"), Path::new("/r/run"));
        assert_eq!(get(&vars, "HOME"), Some("/r/home"));
        assert_eq!(get(&vars, "XDG_CONFIG_HOME"), Some("/r/config"));
        assert_eq!(get(&vars, "XDG_DATA_DIRS"), Some("/r/none"));
        assert_eq!(get(&vars, "XDG_CONFIG_DIRS"), Some("/r/none"));
        assert_eq!(get(&vars, "XDG_RUNTIME_DIR"), Some("/r/run"));
        assert_eq!(get(&vars, "TMPDIR"), Some("/r"));
        assert_eq!(get(&vars, "PATH"), Some("/usr/bin:/bin"));
        assert_eq!(
            get(&vars, "DBUS_SYSTEM_BUS_ADDRESS"),
            Some("unix:path=/r/run/no-system-bus")
        );
    }

    #[test]
    fn set_replaces_or_appends() {
        let mut vars = env_list(Path::new("/r"), Path::new("/r/run"));
        let n = vars.len();
        set(&mut vars, "HOME", "/elsewhere");
        set(&mut vars, "GIT_AUTHOR_NAME", "x");
        assert_eq!(vars.len(), n + 1);
        assert_eq!(get(&vars, "HOME"), Some("/elsewhere"));
        assert_eq!(get(&vars, "GIT_AUTHOR_NAME"), Some("x"));
    }

    #[test]
    fn the_short_link_is_short_stable_and_per_root() {
        let long =
            Path::new("/home/someone/rs-wt/lane/tmp/agent-live.bkxE6l/out/scratch/world-5Vb5Za");
        let name = short_run(long);
        assert_eq!(name, short_run(long));
        assert_ne!(name, short_run(Path::new("/elsewhere/world-5Vb5Za")));
        assert_eq!(
            short_run(Path::new("/tmp/scratch-root")),
            PathBuf::from("/tmp/dl-c037e38128f29d69")
        );
        let socket = name.join("bardo/inference/vllm-qwen3-4b-instruct-2507-fp8.sock");
        assert!(socket.as_os_str().len() < 108, "{}", socket.display());
    }

    /// `xtask/test-env.sh` and this list are one list: every variable the script exports is one of
    /// ours, and every one it unsets is one we never set.
    #[test]
    fn the_script_and_the_list_agree() {
        let script = std::fs::read_to_string(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../../xtask/test-env.sh"
        ))
        .expect("xtask/test-env.sh");
        let ours = env_list(Path::new("/r"), Path::new("/r/run"));
        let mut exported = 0;
        for line in script.lines() {
            if let Some(rest) = line.strip_prefix("export ") {
                let name = rest.split('=').next().expect("a name");
                assert!(get(&ours, name).is_some(), "test-env.sh exports {name}");
                exported += 1;
            } else if let Some(rest) = line.strip_prefix("unset ") {
                for name in rest.split_whitespace() {
                    assert!(get(&ours, name).is_none(), "test-env.sh unsets {name}");
                }
            }
        }
        assert!(exported >= 9, "the script's exports were not read");
    }
}
