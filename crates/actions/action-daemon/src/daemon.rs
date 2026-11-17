//! The running daemon: the files it reads, the seams it builds over the session bus, the router
//! over them, the bus served, the audit queue drained and the person's logout watched.

use crate::acp_gate::AcpGate;
use crate::audit::AuditLog;
use crate::builtin::{HostedLink, builtin_manifests};
use crate::builtin_checkpoints::CheckpointsPort;
use crate::builtin_companion::CompanionPort;
use crate::callers;
use crate::config::{ConfigError, IntentdConfig};
use crate::floor::{FLOOR_FILE, read_floor};
use crate::grants::FileGrants;
use crate::index_files::IndexStore;
use crate::infer::{InferdWriter, ReaderClient};
use crate::link::DbusLink;
use crate::logout::watch_logind;
use crate::manifests::{Loaded, intents_dir, load_manifests};
use crate::policies::{ADMIN_DIR, PolicyDirs, load_policies};
use crate::policies_watch::follow as follow_policies;
use crate::reach::{LiveReach, follow as follow_reach};
use crate::reviewers::reviewer;
use crate::search_files::ship_search_files;
use crate::serve::{ServeFault, closed, serve_on_gated};
use crate::settings_watch::{SettingsWatch, WatchState, apply, apply_next};
use crate::sheet::SheetConfirmer;
use crate::signals::{Cadence, pump};
use crate::space_watch::SpaceKeeper;
use crate::system::{DaemonLog, SystemSeams};
use account_callers::ProcRoot;
use action_checkpoint_git::GitStore;
use action_core::AuditRecord;
use action_dbus::BusConnection;
use action_dbus::InferWatch;
use action_dbus::callers::{CallerTable, Identity};
use action_router::{Router, Seams};
use action_settings::{AgentSettings, Locator, REVIEW_CEILING};
use action_store::SystemClock;
use agent_skills::{Roots, discover};
use agent_skills_disk::DiskFolder;
use base_path::AbsPath;
use base_xdg::{PathError, Xdg};
use companion_memory::{AlmanacMemory, AlmanacSessionLog};
use companion_memory_core::QueuedSink;
use permission_core::Floor;
use provenance::SpaceId;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

/// How often the audit queue is drained into memoryd, unless the setup says otherwise.
const DRAIN_EVERY: Duration = Duration::from_secs(5);

/// How often the restore points past their keeping are forgotten (once at start, then daily).
const PRUNE_EVERY: Duration = Duration::from_hours(24);

/// How long the daemon waits before following accountd's Spaces again after losing it.
const SPACES_RETRY: Duration = Duration::from_secs(10);

/// What only the acceptance harness's mains choose: where callers' cgroups are read from and
/// whether the inferd watch writes to a file.
///
/// The shipped `main` passes the default (the system's `/proc`, no watch) and no environment
/// variable can change either.
#[derive(Debug, Clone)]
pub struct HarnessSeams {
    /// Where callers' cgroups are read.
    pub proc_root: ProcRoot,
    /// The system's callers file, when the harness's main chose another than `/etc/bardo`'s: the
    /// harness is its own system, and a fixed role (the shell's) comes from no other file.
    pub system_callers: Option<PathBuf>,
    /// What is kept of the exchanges with inferd.
    pub trace: InferWatch,
    /// The administrator's floor file, when the harness's main chose another than `/etc/bardo`'s.
    pub floor: Option<PathBuf>,
    /// The administrator's policy directory, when the harness's main chose another than
    /// `/etc/bardo/permissions`.
    pub admin_permissions: Option<PathBuf>,
}

impl Default for HarnessSeams {
    fn default() -> Self {
        Self {
            proc_root: ProcRoot::Unset,
            system_callers: None,
            trace: InferWatch::off(),
            floor: None,
            admin_permissions: None,
        }
    }
}

/// Why the daemon did not start or stopped.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum DaemonFault {
    /// `actiond.toml` is not a configuration.
    #[error("{0}")]
    Config(ConfigError),
    /// The bus.
    #[error("{0}")]
    Serve(ServeFault),
    /// The policy set does not load: the shipped files, or an administrator's policy file, which
    /// is never skipped (a daemon that did would leave agents looser than the administrator meant).
    #[error("policy: {0}")]
    Policy(String),
    /// A directory the daemon keeps its files in is not an absolute path.
    #[error("not an absolute path: {0}")]
    State(String),
    /// `actiond-callers.toml` exists and cannot be used. A daemon that dropped it would identify
    /// fewer or more callers than it was told to.
    #[error("{0}")]
    Callers(String),
    /// The floor file exists and cannot be used. A daemon that skipped it would leave agents
    /// looser than the administrator meant.
    #[error("{0}")]
    Floor(String),
    /// A background duty ended while the daemon was serving: the daemon looks healthy to its
    /// callers but that work has stopped, so it exits and the unit restarts it.
    #[error("background duty `{duty}` stopped: {why}")]
    Duty {
        /// Which duty.
        duty: &'static str,
        /// Why it ended: its panic message, or that it returned.
        why: String,
    },
}

/// What a daemon starts from: the files it reads, found through the environment.
#[derive(Debug, Clone)]
pub struct Setup {
    /// The configuration.
    pub config: IntentdConfig,
    /// The installed manifests.
    pub manifests: Loaded,
    /// The consent file.
    pub grants: PathBuf,
    /// Where the private files of the restore points are kept (`bardo/actions/checkpoints` under the
    /// person's data directory, beside the consent file): never inside a repository.
    pub checkpoints: PathBuf,
    /// The person's login session, when known (`XDG_SESSION_ID`).
    pub session: Option<String>,
    /// Where what the apps pushed to the index is kept between logins.
    pub index: IndexStore,
    /// How often the audit queue is written to memoryd.
    pub audit_every: Duration,
    /// The data directories the manifests are read from, rescanned while the daemon runs.
    pub data_dirs: Vec<PathBuf>,
    /// How often the router's state is looked at for signals, and the manifests rescanned.
    pub signals: Cadence,
    /// Who is which unit: `actiond-callers.toml` of `/etc/bardo` with the person's laid over it.
    pub callers: CallerTable,
    /// Says who a connection is, when a test names its own (`Identity::introduce_app`); otherwise
    /// porter's `ProcCallers` over `callers` and `proc_root`.
    pub identity: Option<Arc<Identity>>,
    /// Where callers' cgroups are read: `/proc`, unless the harness's main passed another root.
    pub proc_root: ProcRoot,
    /// Where the person's settings file is (`actions/settings.toml`, design/22 section 3.27).
    pub settings: Locator,
    /// What is kept of the exchanges with inferd (off, unless the harness's main passed a watch).
    pub trace: InferWatch,
    /// The administrator's floor: none unless the file exists (`run_with` reads it).
    pub floor: Floor,
    /// Where the policy files are read from, and watched.
    pub permissions: PolicyDirs,
}

/// `$XDG_DATA_HOME` then `$XDG_DATA_DIRS`, with their defaults.
///
/// # Errors
/// Neither `XDG_DATA_HOME` nor `HOME` is an absolute path.
pub fn data_dirs(env: &impl Fn(&str) -> Option<String>) -> Result<Vec<PathBuf>, PathError> {
    let xdg = Xdg::new(env);
    let mut dirs = vec![xdg.data_home()?];
    dirs.extend(xdg.data_dirs());
    Ok(dirs)
}

/// The directory the daemon keeps its files in could not be found.
fn state(why: PathError) -> DaemonFault {
    DaemonFault::State(why.to_string())
}

impl Setup {
    /// Reads the configuration (the first `bardo/actions/actiond.toml` of the configuration
    /// directories, else the shipped one) and the manifests of the data directories.
    ///
    /// # Errors
    ///
    /// Returns a [`DaemonFault`] when a directory is not absolute, the configuration or the callers
    /// file cannot be used, or the policy set does not load.
    pub fn from_env(env: &impl Fn(&str) -> Option<String>) -> Result<Self, DaemonFault> {
        let xdg = Xdg::new(env);
        let config_home = xdg.config_home().map_err(state)?;
        let found = std::iter::once(config_home.clone())
            .chain(xdg.config_dirs())
            .map(|d| d.join("bardo").join("actions").join("actiond.toml"))
            .find_map(|path| std::fs::read_to_string(path).ok());
        let config = match found {
            Some(text) => IntentdConfig::parse(&text),
            None => IntentdConfig::shipped(),
        }
        .map_err(DaemonFault::Config)?;
        let callers = callers::read(env).map_err(|e| DaemonFault::Callers(e.to_string()))?;
        let data = data_dirs(env).map_err(state)?;
        let permissions = PolicyDirs {
            data_home: data[0].clone(),
            system_data: data[1..].to_vec(),
            person: config_home
                .join("bardo")
                .join("actions")
                .join("permissions"),
            admin: PathBuf::from(ADMIN_DIR),
        };
        let grants = intents_dir(&data[0]).join("grants.json");
        Ok(Self {
            config,
            manifests: load_manifests(&data),
            grants,
            checkpoints: data[0].join("bardo").join("actions").join("checkpoints"),
            session: env("XDG_SESSION_ID").filter(|v| !v.is_empty()),
            index: IndexStore::under(&data[0]),
            audit_every: DRAIN_EVERY,
            data_dirs: data,
            signals: Cadence::default(),
            callers,
            identity: None,
            proc_root: ProcRoot::Unset,
            settings: Locator::from_env(env),
            trace: InferWatch::off(),
            floor: Floor::none(),
            permissions,
        })
    }
}

/// A daemon that is serving.
#[derive(Debug)]
pub struct Running {
    tasks: Duties,
    session: BusConnection,
}

/// The daemon's background duties, each under a name. None is meant to end while the daemon runs.
#[derive(Debug, Default)]
struct Duties {
    set: tokio::task::JoinSet<()>,
    names: std::collections::HashMap<tokio::task::Id, &'static str>,
}

impl Duties {
    fn spawn(&mut self, name: &'static str, duty: impl Future<Output = ()> + Send + 'static) {
        let id = self.set.spawn(duty).id();
        self.names.insert(id, name);
    }

    /// Waits for the first duty to end, however it ends, and says which and why. With no duty
    /// left it never finishes.
    async fn first_ended(&mut self) -> DaemonFault {
        match self.set.join_next_with_id().await {
            None => std::future::pending().await,
            Some(Ok((id, ()))) => self.fault(id, "it returned".into()),
            Some(Err(error)) => {
                let why = if error.is_panic() {
                    format!("it panicked: {error}")
                } else {
                    format!("it was cancelled: {error}")
                };
                self.fault(error.id(), why)
            }
        }
    }

    fn fault(&mut self, id: tokio::task::Id, why: String) -> DaemonFault {
        let duty = self.names.remove(&id).unwrap_or("unknown");
        DaemonFault::Duty { duty, why }
    }
}

impl Running {
    /// Waits until the session connection closes.
    pub async fn closed(&self) {
        closed(&self.session).await;
    }

    /// Waits until a background duty ends, which it should never do: says which and why.
    pub async fn duty_ended(&mut self) -> DaemonFault {
        self.tasks.first_ended().await
    }

    /// Stops serving: the background tasks end and the connection closes, so the name is free.
    pub async fn stop(mut self) {
        self.tasks.set.abort_all();
        let _ = self.session.close().await;
    }
}

/// Builds the router over the seams of `session` and serves `org.bardo.Intents1` on it.
///
/// It drains the audit queue and, when `system` is given, ends the terminal's grants when logind
/// removes the person's session.
///
/// # Errors
///
/// Returns a [`DaemonFault`] when the bus name cannot be served or a background duty cannot start.
#[expect(
    clippy::too_many_lines,
    reason = "one start-up sequence: each step needs what the one before made, so there is no part to lift out"
)]
pub async fn start(
    session: &BusConnection,
    system: Option<&BusConnection>,
    setup: Setup,
) -> Result<Running, DaemonFault> {
    let Setup {
        config,
        manifests,
        grants,
        checkpoints,
        index,
        session: login,
        audit_every,
        data_dirs,
        signals,
        callers,
        identity,
        proc_root,
        settings,
        trace,
        floor,
        permissions,
    } = setup;
    for (file, why) in &manifests.skipped {
        eprintln!("intentd: skipped {}: {why}", file.display());
    }
    // The ACP name plays its roles only while the setting is on; the settings watch below keeps the
    // gate current, for the bus (who is an editor) and for the sheets (whose process is trusted).
    let acp = AcpGate::shut();
    let who = identity.unwrap_or_else(|| {
        Arc::new(Identity::on(
            session.clone(),
            callers,
            proc_root.honoured().map(std::path::PathBuf::as_path),
        ))
    });
    let confirmer =
        SheetConfirmer::trusting(session.clone(), Arc::new(config.clone()), Arc::clone(&who))
            .gated(acp.clone(), &proc_root);
    let port = CompanionPort::new();
    let restore_port = CheckpointsPort::new();
    let link = HostedLink::new(
        DbusLink::new(session.clone()),
        memory_client::DbusTransport::new(session.clone()),
        port.clone(),
        restore_port.clone(),
    )
    .map_err(|e| DaemonFault::Policy(e.to_string()))?;
    let kept = AbsPath::parse(&checkpoints.to_string_lossy())
        .map_err(|e| DaemonFault::State(format!("{}: {e}", checkpoints.display())))?;
    let seams = SystemSeams {
        link,
        confirmer,
        // The router enforces the person's review times, live; the cascade's own are the ceiling.
        reviewer: reviewer(session, config.reviewers.as_ref(), REVIEW_CEILING, &trace)
            .ok_or_else(|| DaemonFault::Policy("no reviewer set".into()))?,
        grants: FileGrants::at(grants)
            .with_defaults(&data_dirs)
            .with_rules(permissions.clone()),
        sink: QueuedSink::new(),
        clock: SystemClock,
        memory: AlmanacMemory::over(memory_client::DbusTransport::new(session.clone())),
        writer: InferdWriter::on_bus(session, trace.clone()),
        reader: ReaderClient::new(session.clone()),
        log: DaemonLog::Almanac(AlmanacSessionLog::over(
            memory_client::DbusTransport::new(session.clone()),
            SpaceId::desktop(),
            SystemClock,
        )),
        checkpoints: GitStore::new(kept),
    };
    // The policy files, layered: a bad person or assistant file is skipped and a bad preset falls
    // back to the built-in one (both logged); a bad administrator file stops the daemon.
    let loaded = load_policies(&permissions).map_err(|e| DaemonFault::Policy(e.to_string()))?;
    for rejected in &loaded.skipped {
        eprintln!("intentd: skipped policy {rejected}");
    }
    // The Settings page has no read-only row, so where the preset came from is said here.
    eprintln!("intentd: permissions: {}", loaded.preset);
    for dir in &loaded.unused {
        eprintln!("intentd: unused policy folder {}", dir.display());
    }
    let mut router = Router::new(seams, config.agent, loaded.presets);
    router.floor = floor;
    // The person's settings over actiond.toml's proposed values, now and whenever the file changes.
    let mut watched = SettingsWatch::start(settings, AgentSettings::over(config.agent));
    if let WatchState::Blind { reason } = watched.state() {
        eprintln!("intentd: settings are read once, not watched: {reason}");
    }
    apply(&router, &watched.current());
    {
        let mut state = router
            .state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        for manifest in builtin_manifests()
            .map_err(|e| DaemonFault::Policy(e.to_string()))?
            .into_iter()
            .chain(manifests.manifests)
        {
            state.registry.insert(manifest);
        }
    }
    // What the apps ship and what they pushed in earlier logins is searchable from the start.
    ship_search_files(&router, &data_dirs);
    index.restore(&router);
    // The skills the data directories ship: valid files only; which are offered is checked against
    // the registry at each load. What did not load, or is hidden for a missing action, is logged.
    let found = discover(&Roots::from_dirs(&data_dirs), &DiskFolder);
    for rejected in &found.rejected {
        eprintln!(
            "intentd: skill {}: {}",
            rejected.dir.display(),
            rejected.fault
        );
    }
    router.install_skills(found.skills);
    router
        .hidden_skills()
        .iter()
        .for_each(|line| eprintln!("intentd: {line}"));
    // A session the log already holds keeps its name: nothing opened from now on takes it. Each
    // is restored when a request names it. Without memoryd the log is unreadable and the daemon
    // starts anyway; sessions then cannot be recorded or restored.
    if let Err(why) = router.adopt_sessions().await {
        eprintln!("intentd: the session log is not usable ({why}): sessions are not recorded");
        router.seams.log = DaemonLog::Off(action_router::NoLog);
    }
    let router = Arc::new(router);
    port.attach(&router);
    restore_port.attach(&router);
    acp.set(watched.current().value.acp);
    acp.set_agents(watched.current().value.agents);
    serve_on_gated(
        session,
        Arc::clone(&router),
        Arc::new(config),
        who,
        &proc_root,
        acp.clone(),
        Some(index),
    )
    .await
    .map_err(DaemonFault::Serve)?;
    let mut tasks = Duties::default();
    let followed = Arc::clone(&router);
    let blind = matches!(watched.state(), WatchState::Blind { .. });
    tasks.spawn("settings", async move {
        while let Some(loaded) = apply_next(&followed, &mut watched).await {
            acp.set(loaded.value.acp);
            acp.set_agents(loaded.value.agents);
        }
        // A watch that could not start was announced at startup: its settings stay as first
        // read, which is no failure. A live watch that ends is one.
        if blind {
            std::future::pending::<()>().await;
        }
    });
    let live = LiveReach::none();
    tasks.spawn(
        "reach",
        follow_reach(
            Arc::clone(&router),
            session.clone(),
            data_dirs.clone(),
            live.clone(),
        ),
    );
    tasks.spawn(
        "signals",
        pump(
            Arc::clone(&router),
            session.clone(),
            data_dirs,
            signals,
            live,
        ),
    );
    tasks.spawn(
        "spaces",
        SpaceKeeper::new(Arc::clone(&router), false, SPACES_RETRY).run(session.clone()),
    );
    tasks.spawn("checkpoints", prune_checkpoints(Arc::clone(&router)));
    tasks.spawn(
        "policies",
        follow_policies(Arc::clone(&router), permissions),
    );
    let queue = Arc::clone(&router);
    let mut audit = AuditLog::over(memory_client::DbusTransport::new(session.clone()));
    tasks.spawn("audit", async move {
        loop {
            tokio::time::sleep(audit_every).await;
            let report = audit
                .flush(&queue.seams.sink, |record| placed_by(&queue, record))
                .await;
            count_lost(&queue, report.lost);
        }
    });
    if let Some(system) = system {
        // Subscribed before `start` returns: a logout after that is never missed.
        match watch_logind(system).await {
            Ok(logouts) => {
                let router = Arc::clone(&router);
                tasks.spawn("logind", logouts.end_terminals(router, login));
            }
            Err(why) => eprintln!("intentd: not watching logind: {why}"),
        }
    }
    Ok(Running {
        tasks,
        session: session.clone(),
    })
}

/// Forgets the restore points that are past their keeping, now and then every day: the newest
/// few of each session stay, and nothing older than the days the person keeps them.
async fn prune_checkpoints<S: Seams + 'static>(router: Arc<Router<S>>) {
    loop {
        let forgotten = router.prune_checkpoints().await;
        if forgotten.0 > 0 {
            eprintln!("intentd: forgot {} old restore points", forgotten.0);
        }
        tokio::time::sleep(PRUNE_EVERY).await;
    }
}

/// Adds records the log lost to the counter `Control.State` shows.
fn count_lost<S: Seams>(router: &Router<S>, lost: usize) {
    if lost == 0 {
        return;
    }
    let mut state = router
        .state
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let more = u32::try_from(lost).unwrap_or(u32::MAX);
    state.kill.audit_lost = base_ids::Count(state.kill.audit_lost.0.saturating_add(more));
}

/// Where a record that names no Space of its own happened, from the router's session and task
/// tables: a breaker trip is the session's, a task policy the task's, a halt the Space it covers.
fn placed_by<S: Seams>(router: &Router<S>, record: &AuditRecord) -> Option<SpaceId> {
    let state = router
        .state
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    match record {
        AuditRecord::Breaker { session, .. } | AuditRecord::BriefConfirmed { session, .. } => {
            state.sessions.get(session).map(|r| r.space.clone())
        }
        AuditRecord::TaskPolicy { task, .. } | AuditRecord::PolicyCorrected { task, .. } => {
            state.tasks.get(task).map(|t| t.space.clone())
        }
        AuditRecord::Halt {
            scope: provenance::SpaceScope::Only(space),
            ..
        } => Some(space.clone()),
        _ => None,
    }
}

/// The daemon: the session bus, the system bus for logind (absent, it is not watched), the
/// files `env` names (the process's environment, which its `main` passes in). Returns when the bus
/// closes.
///
/// # Errors
///
/// Returns a [`DaemonFault`] when setup fails (configuration, callers, floor or policies) or the
/// bus fails while serving.
pub async fn run(env: &impl Fn(&str) -> Option<String>) -> Result<(), DaemonFault> {
    run_with(HarnessSeams::default(), env).await
}

/// [`run`] with the seams the acceptance harness's mains choose.
///
/// # Errors
///
/// Returns a [`DaemonFault`] when setup fails (configuration, callers, floor or policies) or the
/// bus fails while serving.
pub async fn run_with(
    seams: HarnessSeams,
    env: &impl Fn(&str) -> Option<String>,
) -> Result<(), DaemonFault> {
    let mut setup = Setup::from_env(env)?;
    setup.proc_root = seams.proc_root;
    setup.trace = seams.trace;
    let floor = seams.floor.unwrap_or_else(|| PathBuf::from(FLOOR_FILE));
    setup.floor = read_floor(&floor).map_err(|e| DaemonFault::Floor(e.to_string()))?;
    setup.permissions.admin = seams
        .admin_permissions
        .unwrap_or_else(|| PathBuf::from(ADMIN_DIR));
    if let Some(system) = &seams.system_callers {
        setup.callers =
            callers::read_at(system, env).map_err(|e| DaemonFault::Callers(e.to_string()))?;
    }
    let session = dbus_kit::connect::session_from_env(env)
        .await
        .map_err(|e| DaemonFault::Serve(ServeFault::Bus(e.to_string())))?;
    let system = BusConnection::system().await.ok();
    if system.is_none() {
        eprintln!("intentd: no system bus: the terminal's grants end only when intentd does");
    }
    let mut running = start(&session, system.as_ref(), setup).await?;
    tokio::select! {
        () = closed(&session) => {
            running.stop().await;
            Ok(())
        }
        fault = running.duty_ended() => {
            eprintln!("intentd: {fault}: exiting so the unit restarts the daemon");
            running.stop().await;
            Err(fault)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A duty that panics ends the wait with an error that names it.
    #[tokio::test]
    async fn a_panicking_duty_is_named() {
        let mut duties = Duties::default();
        duties.spawn("quiet", std::future::pending());
        duties.spawn("broken", async { panic!("boom") });
        let fault = duties.first_ended().await;
        assert!(
            matches!(&fault, DaemonFault::Duty { duty: "broken", why } if why.contains("boom")),
            "{fault:?}"
        );
    }

    /// A duty that returns, which none should, is named too.
    #[tokio::test]
    async fn a_duty_that_returns_is_named() {
        let mut duties = Duties::default();
        duties.spawn("quiet", std::future::pending());
        duties.spawn("done", async {});
        let fault = duties.first_ended().await;
        assert!(
            matches!(&fault, DaemonFault::Duty { duty: "done", .. }),
            "{fault:?}"
        );
    }
}
