//! Who is calling: a bus connection's unique name to a [`Caller`].
//!
//! The caller is derived by the transport, never sent: the bus says which process owns a
//! connection, and `account_callers::ProcCallers` (shared with accountd) names it. inferd's own
//! caller table, the system file `/etc/bardo/inferd-callers.toml`
//! (`account_callers::INFERD_CALLERS`, which a person cannot write; never `inferd.toml`), is rows of
//! that shared table, keyed by systemd unit (the programs a unit runs are `account_callers::CallerRow::images`:
//! daemons by convention, the Settings app and the shell in `bin`): the computer-use daemon's units with the `ComputerUse` role,
//! each app's with `App`. This is advisory for unsandboxed processes (the accounts service R12): a
//! process that can start an allowed unit or app scope can be that caller. The computer-use daemon is a fixed unit and
//! the only caller that may open a computer-use session; an app is whichever unit the table
//! names for it, or any identified app scope.

use account_callers::ProcRoot;
use account_callers::{Caller, CallerRow, Install, ProcCallers, installed};
use account_core::AppName;
use account_dbus::BusConnection;
use caller_role_types::CallerRole;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::sync::{Mutex, PoisonError};

/// The app the actions service's companion daemon is: with Settings and the shell, the callers of `Places`.
const COMPANION_APP: &str = "org.bardo.Companion";

/// Why a caller table's text was refused.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum TableError {
    /// Not valid TOML of the table's shape.
    #[error("{0}")]
    Toml(#[from] toml::de::Error),
    /// A `settings` entry that is neither a `.service` unit nor an app name.
    #[error("settings: `{0}` is neither a .service unit nor an app name")]
    BadSettingsEntry(String),
    /// A `shell` entry that is not a `.service` unit: a scope has no main process to check.
    #[error("shell: `{0}` is not a .service unit")]
    BadShellEntry(String),
    /// A `places` entry that is not a unit of an app in `[callers.apps]`.
    #[error("places: `{0}` is not a unit listed under [callers.apps]")]
    PlacesNotAnApp(String),
}

/// What a caller may ask inferd for, over the shared roles ([`CallerRole`]): every role not named
/// here asks for models as a plain app does.
pub trait CallerMay {
    /// Whether it may list the places: Settings, the shell, the companion, and the computer-use daemon, which reads
    /// what the ready models take in to say plainly when none of them can see the screen.
    fn may_list_places(&self) -> bool;
    /// Whether it may list the local engine kinds: Settings, the shell and the companion.
    fn may_list_engines(&self) -> bool;
    /// Whether it may choose where the assistant runs.
    fn may_choose_places(&self) -> bool;
    /// Whether it may set the models Automatic chooses among for a helper: Settings and the
    /// shell, where the Helpers window lives. No other inferd setting is the shell's.
    fn may_write_helper_pools(&self) -> bool;
    /// Whether it may list the computers that ask to use this one and answer one that is
    /// asking: Settings and the shell.
    fn may_answer_guests(&self) -> bool;
}

impl CallerMay for Caller {
    fn may_list_places(&self) -> bool {
        match self.role {
            CallerRole::ComputerUse => true,
            _ => self.may_list_engines(),
        }
    }

    fn may_list_engines(&self) -> bool {
        match self.role {
            CallerRole::Settings | CallerRole::SheetHost => true,
            CallerRole::Agent => self.app.name.as_str() == COMPANION_APP,
            _ => false,
        }
    }

    fn may_choose_places(&self) -> bool {
        self.role == CallerRole::Agent
    }

    fn may_write_helper_pools(&self) -> bool {
        matches!(self.role, CallerRole::Settings | CallerRole::SheetHost)
    }

    fn may_answer_guests(&self) -> bool {
        matches!(self.role, CallerRole::Settings | CallerRole::SheetHost)
    }
}

/// Which systemd unit is which caller.
///
/// ```toml
/// Computer use = ["bardo-computer-use.service"]
/// settings = ["org.bardo.Settings.service"]
/// [apps]
/// "org.bardo.Memory" = ["bardo-memoryd.service"]
/// "org.bardo.Companion" = [{ unit = "bardo-agent-acp.scope", exe = ["/usr/bin/bardo-agent-acp"] }]
/// ```
///
/// An `[apps]` or `agent_launcher` entry may name the programs that run in its unit ([`Unit`]):
/// a unit whose name gives no program (a scope, a dotted service) is nobody without them.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct CallerTable {
    #[serde(default)]
    computer_use: BTreeSet<String>,
    /// The units of the agent launcher.
    #[serde(default)]
    agent_launcher: BTreeSet<Unit>,
    #[serde(default)]
    apps: BTreeMap<AppName, BTreeSet<Unit>>,
    /// The Settings app's units: the only callers of the settings module, each only as its unit's
    /// main process (as accountd's `callers.toml` row for Settings). An entry that is an app name
    /// (the form this table had, `"org.bardo.Settings"`) stands for that app's unit,
    /// `<app>.service`: never for any process in an app scope of that name, which any program can
    /// start.
    #[serde(default)]
    settings: BTreeSet<String>,
    /// The shell's units (`bardo-shell.service`): the shell may list the places and ask for nothing
    /// else. Services only, so the role goes to the unit's main process.
    #[serde(default)]
    shell: BTreeSet<String>,
    /// The units, each already an app under `apps`, that may choose where the assistant runs
    /// (`places` in the options of `Open`, `Prepare` and `Availability`).
    #[serde(default)]
    places: BTreeSet<String>,
}

/// The unit and the app a `settings` entry stands for: a `.service` unit is the Settings app's;
/// an app name is that app's `<app>.service`. Anything else is no entry.
fn settings_unit(entry: &str) -> Option<(String, AppName)> {
    match entry.strip_suffix(".service") {
        Some(name) if !name.is_empty() => {
            Some((entry.to_owned(), AppName::parse(SETTINGS_APP).ok()?))
        }
        Some(_) => None,
        None => AppName::parse(entry)
            .ok()
            .map(|app| (format!("{entry}.service"), app)),
    }
}

impl CallerTable {
    /// A table naming the computer-use daemon's units and each app's.
    #[must_use]
    pub fn new(computer_use: BTreeSet<String>, apps: BTreeMap<AppName, BTreeSet<String>>) -> Self {
        Self {
            computer_use,
            agent_launcher: BTreeSet::new(),
            apps: apps
                .into_iter()
                .map(|(app, units)| (app, units.into_iter().map(Unit::from).collect()))
                .collect(),
            settings: BTreeSet::new(),
            shell: BTreeSet::new(),
            places: BTreeSet::new(),
        }
    }

    /// The same table, naming `units` as the shell's.
    #[must_use]
    pub fn with_shell(self, units: BTreeSet<String>) -> Self {
        Self {
            shell: units,
            ..self
        }
    }

    /// The same table, naming `units` (each also an app's unit under `apps`) as callers that
    /// may choose where the assistant runs.
    #[must_use]
    pub fn with_places(self, units: BTreeSet<String>) -> Self {
        Self {
            places: units,
            ..self
        }
    }

    /// The same table, naming `units` as the agent launcher's.
    #[must_use]
    pub fn with_agent_launcher(self, units: BTreeSet<String>) -> Self {
        Self {
            agent_launcher: units.into_iter().map(Unit::from).collect(),
            ..self
        }
    }

    /// The same table, naming `units` (or app names, each standing for `<app>.service`) as the
    /// Settings app's, the only callers of the settings module.
    #[must_use]
    pub fn with_settings(self, units: BTreeSet<String>) -> Self {
        Self {
            settings: units,
            ..self
        }
    }

    /// The table in TOML text. A `settings` entry that is neither a `.service` unit nor an app
    /// name is refused.
    ///
    /// # Errors
    ///
    /// Returns a [`TableError`] when the text is not a table or a `settings` entry is neither a unit nor
    /// an app name.
    pub fn from_toml_text(text: &str) -> Result<Self, TableError> {
        let table: Self = toml::from_str(text)?;
        if let Some(bad) = table.settings.iter().find(|e| settings_unit(e).is_none()) {
            return Err(TableError::BadSettingsEntry(bad.clone()));
        }
        if let Some(bad) = table.shell.iter().find(|e| !e.ends_with(".service")) {
            return Err(TableError::BadShellEntry(bad.clone()));
        }
        let listed = |unit: &&String| {
            table
                .apps
                .values()
                .any(|units| units.iter().any(|entry| entry.name() == unit.as_str()))
        };
        match table.places.iter().find(|unit| !listed(unit)) {
            Some(bad) => Err(TableError::PlacesNotAnApp(bad.clone())),
            None => Ok(table),
        }
    }

    /// The rows of the shared table this table is: the computer-use daemon's first, so an app entry cannot claim its
    /// unit (the shared table answers with the first row that names one).
    #[must_use]
    pub fn rows(&self) -> account_callers::CallerTable {
        let computer_use_app = AppName::parse(COMPUTER_USE_APP).ok();
        let computer_use = computer_use_app.into_iter().flat_map(|app| {
            self.computer_use.iter().map(move |unit| CallerRow {
                unit: Some(unit.clone()),
                app: app.clone(),
                role: CallerRole::ComputerUse,
                exe: Vec::new(),
                name: None,
            })
        });
        let launcher_app = AppName::parse(LAUNCHER_APP).ok();
        let launcher = launcher_app.into_iter().flat_map(|app| {
            self.agent_launcher.iter().map(move |unit| CallerRow {
                unit: Some(unit.name().to_owned()),
                app: app.clone(),
                role: CallerRole::AgentLauncher,
                exe: unit.exe(),
                name: None,
            })
        });
        // Settings by its unit, whose main process alone has the role (the caller-identity library's `Unit` or `Full` proof):
        // before the apps, so an app entry cannot claim the unit.
        let settings = self
            .settings
            .iter()
            .filter_map(|entry| settings_unit(entry))
            .map(|(unit, app)| CallerRow {
                unit: Some(unit),
                app,
                role: CallerRole::Settings,
                exe: installed(Install::Bin, SETTINGS_PROGRAM),
                name: None,
            });
        // The shell by its scope; the shared role is the one accountd gives it.
        let shell_app = AppName::parse(SHELL_APP).ok();
        let shell = shell_app.into_iter().flat_map(|app| {
            self.shell.iter().map(move |unit| CallerRow {
                unit: Some(unit.clone()),
                app: app.clone(),
                role: CallerRole::SheetHost,
                exe: installed(Install::Bin, SHELL_PROGRAM),
                name: None,
            })
        });
        // An app whose unit is under `places` is one that may choose places, and carries the shared role `Agent`
        // (companiond, readerd and actiond are the ones accountd calls so).
        let apps = self.apps.iter().flat_map(|(app, units)| {
            units.iter().map(move |unit| CallerRow {
                unit: Some(unit.name().to_owned()),
                app: app.clone(),
                role: if self.places.contains(unit.name()) {
                    CallerRole::Agent
                } else {
                    CallerRole::App
                },
                exe: unit.exe(),
                name: None,
            })
        });
        account_callers::CallerTable {
            callers: computer_use
                .chain(launcher)
                .chain(settings)
                .chain(shell)
                .chain(apps)
                .collect(),
        }
    }

    /// The caller that is the unit `unit`, through [`CallerTable::rows`].
    #[must_use]
    pub fn resolve(&self, unit: &str) -> Option<Caller> {
        self.rows().resolve_unit(unit)
    }
}

/// The app the computer-use daemon is.
const COMPUTER_USE_APP: &str = "org.bardo.ComputerUse";

/// The app the agent launcher is.
const LAUNCHER_APP: &str = "org.bardo.AgentLauncher";

/// The Settings app, which a `settings` entry that is a unit runs.
const SETTINGS_APP: &str = "org.bardo.Settings";

/// The shell, which a `shell` entry runs.
const SHELL_APP: &str = "org.bardo.Shell";

/// The programs the Settings app and the shell are installed as (in `bin`): the unit manager must
/// say their units start one of these before they keep a fixed role.
const SETTINGS_PROGRAM: &str = "bardo-settings";
const SHELL_PROGRAM: &str = "bardo-shell";

/// Who a bus connection is.
pub trait Peers: Send + Sync + 'static {
    /// The caller behind the connection `sender` (its unique name), or `None` if it is nobody the
    /// table names.
    fn caller_of(&self, sender: &str) -> impl Future<Output = Option<Caller>> + Send;
}

impl<T: Peers> Peers for std::sync::Arc<T> {
    async fn caller_of(&self, sender: &str) -> Option<Caller> {
        (**self).caller_of(sender).await
    }
}

/// Peers by process: [`ProcCallers`] over the rows of the [`CallerTable`].
#[derive(Debug)]
pub struct ProcPeers {
    callers: std::sync::Arc<ProcCallers>,
}

impl ProcPeers {
    /// Resolves senders on `connection` through `table`, reading the process tree `root` names.
    #[must_use]
    pub fn with_root(connection: BusConnection, table: &CallerTable, root: &ProcRoot) -> Self {
        let rows = table.rows();
        let callers = match root.honoured() {
            #[cfg(feature = "test-proc-root")]
            Some(dir) => ProcCallers::with_proc_root(connection, rows, dir.clone()),
            _ => ProcCallers::new(connection, rows),
        };
        Self {
            callers: std::sync::Arc::new(callers),
        }
    }
}

/// Whether this build honours `BARDO_INFERD_PROC_ROOT`: only with the `test-proc-root` feature.
pub const HONOURS_PROC_ROOT: bool = cfg!(feature = "test-proc-root");

/// Whether this build honours `BARDO_INFERD_CALLERS`: only with the `test-system-callers` feature, which `test-proc-root`
/// implies.
pub const HONOURS_SYSTEM_CALLERS: bool = cfg!(feature = "test-system-callers");

impl Peers for ProcPeers {
    async fn caller_of(&self, sender: &str) -> Option<Caller> {
        account_callers::Callers::caller_of(&self.callers, sender).await
    }
}

/// Peers from a map of unique names, for tests and for hosts that know their clients.
#[derive(Debug, Default)]
pub struct TablePeers(Mutex<BTreeMap<String, Caller>>);

impl TablePeers {
    /// Nobody is known.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Says that the connection `unique_name` is `caller`.
    pub fn introduce(&self, unique_name: &str, caller: Caller) {
        self.0
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .insert(unique_name.to_owned(), caller);
    }
}

impl Peers for TablePeers {
    #[expect(
        clippy::unused_async_trait_impl,
        reason = "the trait method is async; this implementation has nothing to await"
    )]
    async fn caller_of(&self, sender: &str) -> Option<Caller> {
        self.0
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .get(sender)
            .cloned()
    }
}

mod units;

pub use units::Unit;

#[cfg(test)]
mod tests;
