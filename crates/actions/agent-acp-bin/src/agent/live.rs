//! What every session of a live host shares: the performer the router's `Perform` for the
//! pseudo-app reaches, the pseudo-app served on the bus under `org.bardo.AcpAgent`, and the parts
//! a spawner is made of. One agent's session (`host`) and the sessions of many helpers'
//! agents (`helper_host`) both start from here, so both hold an agent to the same rules.

use crate::agent::host::{Hosting, Wiring};
use crate::agent::places::Places;
use crate::agent::provider::{PerformerProvider, Quiet};
use action_client::DbusTransport;
use agent_acp_host::{AcpBackend, EdgeMenu, IntentsCourt, OsFiles, Parts, Performer, ToolsOffer};
use agent_registry::dirs::AgentsDir;
use permission_core::ProgramName;
use sandbox::{BwrapSandbox, Detected, NetworkMode};

use action_dbus::CONFIRM_PATH;
use action_host::EditorDesk;
use action_manifest::ValidManifest;
use action_provider::serve_on_with;
use agent_launch::{
    Accounts, AgentSpawn, AgentsFile, BwrapProcs, ChildProc, Entry, Registry, login_run,
};
use base_path::AbsPath;
use companion_model::helper::{TerminalLine, TerminalLogin, TerminalRun};
use companion_model::{SeenCommand, WatchedAgent};
use permission_core::ACP_AGENT_APP;

use action_store::SystemClock;
use provenance::{HelperId, SessionId, TaskId};
use std::path::PathBuf;
use std::sync::Arc;

const MANIFEST: &str = include_str!("../../../manifests/org.bardo.AcpAgent.toml");

/// The pseudo-app served, and what a backend for any program is made of.
pub struct Live<A: Accounts + 'static> {
    file: Arc<AgentsFile>,
    accounts: Arc<A>,
    sandbox: BwrapSandbox,
    forwarder: AbsPath,
    enter: Option<AbsPath>,
    run_dir: PathBuf,
    registry: Registry<ChildProc>,
    tools: Option<ToolsOffer>,
    performer: Performer<OsFiles, Detected>,
    /// The router.
    pub court: IntentsCourt<DbusTransport>,
    /// The desk the sheets routed to the host land on.
    pub desk: EditorDesk,
    /// Who answers the sheets of an agent's session.
    pub terminal_fallback: bool,
    /// The agents directory a registry agent was installed in; none writes nothing.
    pub agents: Option<AgentsDir>,
    places: Places,
    placeholder: Placeholder,
}

/// Names that are made once, so that what needs one and has no better is never without: the
/// backend's session before the router gives the real one, and a helper's task and program when
/// its card has none that parses (a caller checks a card before it uses it).
struct Placeholder {
    session: SessionId,
    task: TaskId,
    program: ProgramName,
}

/// Why the host could not serve the pseudo-app.
#[derive(Debug, thiserror::Error)]
pub enum LiveError {
    /// No bubblewrap: no agent is started unconfined.
    #[error("no sandbox (bubblewrap) here ({0}): no agent is started unconfined")]
    NoSandbox(sandbox::CannotSandboxError),
    /// The pseudo-app's manifest does not parse.
    #[error("the agent manifest: {0}")]
    Manifest(#[from] action_manifest::ManifestFileError),
    /// The agent host has no app name.
    #[error("no app name for the agent host")]
    NoAppName,
    /// The bus refused the object.
    #[error("{0}")]
    Bus(#[from] zbus::Error),
    /// The pseudo-app's provider could not be served.
    #[error("{app}: {0}", app = ACP_AGENT_APP)]
    Provider(#[from] action_provider::ServeError),
    /// A name made here does not parse.
    #[error("{0}")]
    Invalid(String),
}

impl Placeholder {
    fn new() -> Result<Self, LiveError> {
        let invalid = |e: &dyn std::fmt::Display| LiveError::Invalid(e.to_string());
        Ok(Self {
            session: SessionId::parse("s-0").map_err(|e| invalid(&e))?,
            task: TaskId::parse("helper-task").map_err(|e| invalid(&e))?,
            program: ProgramName::parse("agent").map_err(|e| invalid(&e))?,
        })
    }
}

impl<A: Accounts + 'static> Live<A> {
    /// Serves the pseudo-app on the bus. The performer counts the commands it runs against
    /// `agent_network`, the network of the agent process.
    pub async fn serve(wiring: Wiring<A>, agent_network: NetworkMode) -> Result<Self, LiveError> {
        let Wiring {
            bus,
            accounts,
            file,
            bwrap,
            forwarder,
            enter,
            run_dir,
            registry,
            tools,
            terminal_fallback,
            desk,
            callers,
            agents,
            places,
        } = wiring;
        let found = match &bwrap {
            Detected::Bwrap(found) => found,
            Detected::Missing(why) => {
                return Err(LiveError::NoSandbox(*why));
            }
        };
        let sandbox = found.clone();
        let performer = Performer::new(OsFiles, bwrap)
            .with_agent_network(agent_network)
            .with_command_deadline(agent_acp_core::command_deadline::CommandDeadline::DEFAULT);
        let manifest: ValidManifest = action_manifest::parse(MANIFEST)?;
        let quiet = Quiet(permission_core::acp_agent_app().ok_or(LiveError::NoAppName)?);
        if terminal_fallback {
            // Served before the name is claimed, so the first sheet finds it.
            bus.object_server()
                .at(
                    CONFIRM_PATH,
                    crate::confirm::ConfirmObject::new(
                        desk.clone(),
                        SystemClock,
                        Arc::clone(&callers),
                    ),
                )
                .await?;
        }
        // The pseudo-app's provider answers the same process `Confirm1` does: intentd's unit.
        serve_on_with(
            &bus,
            PerformerProvider::new(manifest, performer.clone()),
            quiet.clone(),
            quiet,
            callers,
        )
        .await?;
        let court = IntentsCourt::over(DbusTransport::new(bus));
        Ok(Self {
            file,
            accounts,
            sandbox,
            forwarder,
            enter,
            run_dir,
            registry,
            tools,
            performer,
            court,
            desk,
            terminal_fallback,
            agents,
            places,
            placeholder: Placeholder::new()?,
        })
    }

    /// Where this computer keeps what the host needs.
    pub fn places(&self) -> &Places {
        &self.places
    }

    /// The entry of `agents.toml` that runs `program`.
    pub fn entry(&self, program: &ProgramName) -> Option<&Entry> {
        self.file.get(program)
    }

    /// The listed app and conversation `seen` picks up, when it is one of the list's.
    pub fn recognise(&self, seen: &SeenCommand) -> Option<WatchedAgent> {
        self.file.recognise(seen)
    }

    /// The run of `program`'s sign-in in a terminal, resolved from its entry: the command as the
    /// list names it with the login added, the folder, and the same command inside the sandbox
    /// this host puts agents in. None when the list does not have the program, the folder is no
    /// path, or the sandbox will not build the command (nothing is then run unconfined).
    pub fn terminal_run(
        &self,
        program: &ProgramName,
        login: &TerminalLogin,
        folder: Option<&str>,
    ) -> Option<TerminalRun> {
        let entry = self.entry(program)?;
        let cwd = AbsPath::parse(folder?).ok()?;
        let run = login_run(entry, &login.args, &login.env, &cwd)?;
        let run = match &self.enter {
            Some(enter) => run.with_landlock(enter.clone()),
            None => run,
        };
        let line = BwrapProcs::new(self.sandbox.clone(), self.run_dir.clone())
            .command_line(&run)
            .ok()?;
        let env_of = |pairs: Vec<(String, String)>| pairs.into_iter().collect();
        let plain_env = entry
            .set
            .iter()
            .map(|(name, value)| (name.as_str().to_owned(), value.clone()))
            .chain(login.env.clone())
            .collect();
        Some(TerminalRun {
            plain: TerminalLine {
                program: entry.command.as_str().to_owned(),
                args: entry.args.iter().chain(&login.args).cloned().collect(),
                env: plain_env,
            },
            folder: Some(cwd.as_str().to_owned()),
            confined: Some(TerminalLine {
                program: line.program,
                args: line.args,
                env: env_of(line.env),
            }),
        })
    }

    /// The task a helper's agent session runs as.
    pub fn task_of(&self, helper: &HelperId) -> TaskId {
        self.task_named("helper", helper.as_str())
    }

    /// The task named `kind` and `name` in lower-case letters, digits and dashes.
    pub fn task_named(&self, kind: &str, name: &str) -> TaskId {
        let slug: String = name
            .chars()
            .map(|c| c.to_ascii_lowercase())
            .filter(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || *c == '-')
            .collect();
        TaskId::parse(&format!("{kind}-{slug}")).unwrap_or_else(|_| self.placeholder.task.clone())
    }

    /// The command that picks one conversation up in `program`'s own command-line program, from
    /// its entry's `resume` arguments with `own` (the app's own id for the conversation) put in,
    /// run the way a sign-in is: the plain line, the line in the sandbox, and the folder. None
    /// when the entry has no such arguments or the line cannot be built (nothing is then run
    /// unconfined).
    pub fn resume_run(
        &self,
        program: &ProgramName,
        own: &str,
        folder: Option<&str>,
    ) -> Option<TerminalRun> {
        let login = TerminalLogin {
            args: self.entry(program)?.resume_line(own)?,
            ..TerminalLogin::default()
        };
        self.terminal_run(program, &login, folder)
    }

    /// `program`, or a stand-in when there is none (a native helper's card, which no caller
    /// sends here).
    pub fn program_or_stand_in(&self, program: Option<ProgramName>) -> ProgramName {
        program.unwrap_or_else(|| self.placeholder.program.clone())
    }

    /// The tool edge `program` is offered: none when its entry switches the desktop's actions off
    /// or the host has no bridge to give.
    fn tools_for(&self, program: &ProgramName) -> Option<ToolsOffer> {
        self.file
            .get(program)
            .filter(|entry| entry.tools)
            .and_then(|_| self.tools.clone())
    }

    /// A new backend for `program`, its process not yet started: its own spawner, the performer
    /// every session of this host shares, and a tool edge that offers what `menu` allows.
    pub fn backend(&self, program: &ProgramName, menu: EdgeMenu) -> AcpBackend<Hosting<A>> {
        let spawn = AgentSpawn::new(
            Arc::clone(&self.file),
            Arc::clone(&self.accounts),
            BwrapProcs::new(self.sandbox.clone(), self.run_dir.clone()),
            self.run_dir.clone(),
            self.forwarder.clone(),
            self.registry.clone(),
        );
        let spawn = match &self.enter {
            Some(enter) => spawn.with_landlock(enter.clone()),
            None => spawn,
        };
        AcpBackend::<Hosting<A>>::new(Parts {
            program: program.clone(),
            session: self.placeholder.session.clone(),
            spawn,
            performer: self.performer.clone(),
            court: self.court.clone(),
            tools: self.tools_for(program),
        })
        .with_menu(menu)
    }
}
