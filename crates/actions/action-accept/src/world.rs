//! The acceptance world: a private bus, the real daemons as processes (accountd, inferd and memoryd
//! as the packaged binaries of their repos, actiond, readerd and companiond).
//!
//! And the fakes on the bus around them: the shell's `Confirm1` and a mail provider. The model is
//! inferd's replay engine playing a cassette. Everything is scratch: HOME, every XDG directory and
//! the bus.
//!
//! Teardown is certain: every daemon is a `Reaped` child (killed by PID and waited for on drop,
//! with a watchdog if the test process itself dies) and the bus is `base-testkit`'s. Waits are
//! on bus events (a name appearing); the long bounds exist only to turn a hang into a failure.

//!
//! The parts: `env` (what a daemon's process is given), `proc` (the fake `/proc`), `files` (what
//! the daemons read from the scratch root), `start` (starting a world) and `ops` (what a running
//! world does).

mod env;
mod files;
mod ops;
mod proc;
mod start;

pub use env::{DaemonEnv, extra_env, spawn};
pub use files::{
    ACCOUNTD_CALLERS, INFERD_CALLERS, MEMORY_CALLERS, SPACES, inferd_toml, write,
    write_system_callers,
};
pub use proc::{Cgroup, as_app, link_image, place, restore_unit, unit_main};
pub use start::until_owned;

use crate::confirm::Sheet;
use crate::provider::{AppLog, Focus};
use crate::things::App;
use action_core::ConfirmRequest;
use base_testkit::{PrivateBus, Reaped, TestEnv};
use std::path::{Path, PathBuf};
use std::time::Duration;
use tokio::sync::mpsc;

/// How long a wait for a bus event may take before the test fails instead of hanging.
///
/// Shorter than the 2 minutes nextest allows a test (`.config/nextest.toml`), so the failure names
/// what was waited for instead of being a bare kill.
pub const GIVE_UP: Duration = Duration::from_secs(60);

/// Runs `wait`, failing the test, naming `what` it waited for, if it takes longer than [`GIVE_UP`].
///
/// Every call over the bus and every wait on a peer goes through it: a zbus method call has no
/// timeout of its own, and a peer that never answers would hold the test for ever.
///
/// # Panics
///
/// Panics when `wait` takes longer than [`GIVE_UP`].
pub async fn within<T>(what: &str, wait: impl std::future::Future<Output = T>) -> T {
    tokio::time::timeout(GIVE_UP, wait)
        .await
        .unwrap_or_else(|_| panic!("timed out after {GIVE_UP:?} waiting for {what}"))
}

/// The manifest the mail provider answers for, installed for actiond to read.
pub const MAIL_MANIFEST: &str = include_str!("../../dev/accept/fixtures/org.bardo.Mail.toml");

/// Where the daemon binaries are (`env!("CARGO_BIN_EXE_*")` of the test), and the directory the
/// engines of inferd keep their compile caches in, which only a main or a test names.
#[derive(Debug, Clone)]
pub struct Binaries {
    /// `accept-actiond`.
    pub actiond: PathBuf,
    /// `accept-companiond`.
    pub companiond: PathBuf,
    /// `accept-readerd`.
    pub readerd: PathBuf,
    /// The memory service's packaged `memoryd`, built with `test-keys` (a prebuild of the suite).
    pub memoryd: PathBuf,
    /// The accounts service's packaged `accountd`, built with `test-keys`: it holds the AI keys.
    pub accountd: PathBuf,
    /// The accounts service's packaged `inferd`.
    pub inferd: PathBuf,
    /// `accept-bardo-do`.
    pub bardo_do: PathBuf,
    /// `accept-actions-mcp`: the bridge an external agent starts.
    pub action_mcp: PathBuf,
    /// `accept-bus-relay`: a bus connection that is a process of its own ([`World::connect_as`]).
    pub relay: PathBuf,
    /// A directory shared by every world's inferd for its engines' compile caches (vLLM compiles
    /// for minutes in a fresh one), when the run names one.
    pub engine_cache: Option<PathBuf>,
}

/// The variable that names the directory the engines of inferd share their compile caches in (set
/// by the live scripts).
pub const ENGINE_CACHE_VAR: &str = "BARDO_ACTIONS_LIVE_ENGINE_CACHE";

impl Binaries {
    /// The binaries of a run: the daemons beside `exe` (the program that runs them), and the
    /// accounts, inference and memory daemons `cargo xtask accept agent` (or whoever starts the
    /// run) built beforehand, found with [`accept_kit::bin`]. `env` is the environment
    /// `main` hands in: [`ENGINE_CACHE_VAR`] names the engines' shared compile cache.
    ///
    /// # Panics
    ///
    /// When a prebuilt daemon is not where its `BARDO_ACCEPT_BIN_*` variable says (see
    /// [`accept_kit::bin`]).
    pub fn beside(exe: &Path, env: &impl Fn(&str) -> Option<String>) -> Self {
        // A program the run was told about (`BARDO_ACCEPT_BIN_*`, as `cargo xtask` names the
        // prebuilds), else the one built beside `exe`.
        let beside = |name: &str| {
            env(&accept_kit::bin_var(name))
                .filter(|path| !path.is_empty())
                .map_or_else(|| exe.with_file_name(name), PathBuf::from)
        };
        Self {
            actiond: beside("accept-actiond"),
            companiond: beside("accept-companiond"),
            readerd: beside("accept-readerd"),
            memoryd: accept_kit::bin("bardo-memoryd"),
            accountd: accept_kit::bin("bardo-accountd"),
            inferd: accept_kit::bin("bardo-inferd"),
            bardo_do: beside("accept-bardo-do"),
            action_mcp: beside("accept-actions-mcp"),
            relay: beside("accept-bus-relay"),
            engine_cache: env(ENGINE_CACHE_VAR)
                .filter(|root| !root.is_empty())
                .map(PathBuf::from),
        }
    }

    /// Every program from the prebuilds of the suite (`suites.toml`), found with
    /// [`accept_kit::bin`]: for a test of another package, which has no `CARGO_BIN_EXE_*` of
    /// this one's programs.
    ///
    /// # Panics
    ///
    /// When a prebuilt program is not where its `BARDO_ACCEPT_BIN_*` variable says.
    #[must_use]
    pub fn prebuilt() -> Self {
        Self {
            actiond: accept_kit::bin("accept-actiond"),
            companiond: accept_kit::bin("accept-companiond"),
            readerd: accept_kit::bin("accept-readerd"),
            memoryd: accept_kit::bin("bardo-memoryd"),
            accountd: accept_kit::bin("bardo-accountd"),
            inferd: accept_kit::bin("bardo-inferd"),
            bardo_do: accept_kit::bin("accept-bardo-do"),
            action_mcp: accept_kit::bin("accept-actions-mcp"),
            relay: accept_kit::bin("accept-bus-relay"),
            engine_cache: None,
        }
    }
}

/// The model's script: the text of a cassette file (`dev/accept/cassettes`), written into the
/// scratch config for inferd's replay engine to play.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Cassette(pub &'static str);

/// Where the model's answers come from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ModelSource {
    /// inferd's replay engine plays this cassette (the text of a `.jsonl` file).
    Scripted(String),
    /// A real engine: this is the body of an `inferd.toml` (engines, `[ai]` rows) the owner wrote.
    /// The world appends the `[callers.apps]` table, so the body must not hold one.
    Live(String),
}

/// The person's `agent.acp.expose`, written into the scratch settings file. actiond honours the
/// roles of `org.bardo.Acp` only while it is on.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum AcpSetting {
    /// No settings file: the feature is off.
    #[default]
    Off,
    /// `[agent.acp] expose = true`.
    On,
    /// `[agent.acp] agents = true`: actiond counts the host of an external agent
    /// (`org.bardo.AcpAgent`) and `agents.toml` may start programs.
    Agents,
}

/// The scratch `settings.toml` of a world: the person's ACP setting.
#[must_use]
pub fn settings_toml(acp: AcpSetting) -> String {
    match acp {
        AcpSetting::Off => String::new(),
        AcpSetting::On => "[agent.acp]\nexpose = true\n".to_owned(),
        AcpSetting::Agents => "[agent.acp]\nagents = true\n".to_owned(),
    }
}

/// What a world does besides what its model source says.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Options {
    /// Where the scratch root is made, and kept when the run ends (a live run's traces live in
    /// it). Absent: a temporary directory that goes with the world.
    pub keep_in: Option<PathBuf>,
    /// Whether to turn on the model tap in every daemon (`BARDO_ACTIONS_MODEL_TRACE`): every request a daemon
    /// sends a model and its answer, appended to `<scratch>/model.jsonl`. THIS WRITES PROMPTS TO
    /// DISK, in the scratch root.
    pub tap: bool,
    /// A durable home that already holds the cloud key (`--keys-home`): inferd's keys are the
    /// file in it and its non-secret records are copied into the scratch root, so no key is
    /// typed per world.
    pub keys_home: Option<crate::keys_home::KeysHome>,
    /// A catalogue directory whose `*.toml` entries are copied into the scratch root before
    /// inferd starts (see `catalog`).
    pub catalog: Option<PathBuf>,
    /// Whether inferd's replay engine records the request bodies it is asked, to
    /// `<scratch>/record.jsonl`; a world prints them when it is dropped.
    pub record: bool,
    /// Whether the person switched the ACP edge on.
    pub acp: AcpSetting,
    /// What is open in the app's window, which "this thread" means.
    pub focus: Focus,
    /// The app the world serves.
    pub app: App,
}

/// Copies the run's catalogue into the scratch root; a missing source is a failed start.
///
/// # Panics
///
/// Panics when the catalogue cannot be copied from `from`.
pub fn copy_catalog(from: Option<&Path>, root: &Path) {
    if let Some(from) = from {
        crate::catalog::copy_entries(from, &crate::catalog::world_dir(root))
            .unwrap_or_else(|e| panic!("catalogue {}: {e}", from.display()));
    }
}

/// A world's scratch root: a [`TestEnv`] (scratch HOME, XDG tree and runtime directory), and, when
/// the run asked to keep it, a copy of what it holds under that directory when the world ends.
///
/// The copy leaves out what is not a regular file or a directory (the sockets under `run`).
#[derive(Debug)]
pub struct Root {
    // Declared first: `Drop` below copies, and then the environment removes itself.
    keep_in: Option<PathBuf>,
    env: TestEnv,
}

impl Root {
    /// The root of `env` (made with `TestEnv::new`, its buses pointed at the world's); `keep_in` is
    /// where a copy of it is left when this drops.
    #[must_use]
    pub fn new(env: TestEnv, keep_in: Option<&Path>) -> Self {
        Self {
            keep_in: keep_in.map(Path::to_owned),
            env,
        }
    }

    /// The environment of a process started in this root.
    #[must_use]
    pub fn env(&self) -> &TestEnv {
        &self.env
    }

    /// The root.
    #[must_use]
    pub fn path(&self) -> &Path {
        self.env.root()
    }

    fn keep(&self, keep_in: &Path) -> std::io::Result<PathBuf> {
        std::fs::create_dir_all(keep_in)?;
        let to = tempfile::Builder::new()
            .prefix("world-")
            .tempdir_in(keep_in)?
            .keep();
        copy_tree(self.path(), &to)?;
        Ok(to)
    }
}

impl Drop for Root {
    fn drop(&mut self) {
        if let Some(keep_in) = &self.keep_in {
            match self.keep(keep_in) {
                Ok(to) => eprintln!("the scratch root is kept at {}", to.display()),
                Err(e) => eprintln!("the scratch root could not be kept: {e}"),
            }
        }
    }
}

fn copy_tree(from: &Path, to: &Path) -> std::io::Result<()> {
    for entry in std::fs::read_dir(from)? {
        let entry = entry?;
        let kind = entry.file_type()?;
        let target = to.join(entry.file_name());
        if kind.is_dir() {
            std::fs::create_dir_all(&target)?;
            copy_tree(&entry.path(), &target)?;
        } else if kind.is_file() {
            std::fs::copy(entry.path(), &target)?;
        }
    }
    Ok(())
}

/// What a world starts with.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Consent {
    /// The companion's standing consent for Mail's classes in the Space is already on file, so
    /// the only sheet of a run is the one the call itself asks.
    Standing,
    /// No grants on file: the first use of each class asks too.
    FirstUse,
}

/// The running world.
pub struct World {
    // Declared first: the daemons are killed before anything they talk to goes.
    daemons: Vec<(&'static str, Reaped)>,
    /// What the shell's `Confirm1` was shown, and what the person will do with it.
    pub sheet: Sheet,
    /// The same, as a stream: one item per sheet shown.
    pub confirms: mpsc::UnboundedReceiver<ConfirmRequest>,
    /// What the app was asked and what it holds.
    pub app: AppLog,
    /// A connection that owns `org.bardo.Shell` and `org.bardo.Confirm1`: the shell.
    pub shell: zbus::Connection,
    _helpers: Vec<zbus::Connection>,
    /// The relays behind [`World::connect_as`]'s connections, ended with the world.
    relays: std::sync::Mutex<Vec<Reaped>>,
    /// The scratch root.
    pub dir: Root,
    bus: PrivateBus,
}

impl std::fmt::Debug for World {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("World")
    }
}

impl Drop for World {
    fn drop(&mut self) {
        let text = self.recorded_requests();
        if !text.is_empty() {
            eprintln!("---- planner and reader requests (`Options::record`)\n{text}");
        }
    }
}
