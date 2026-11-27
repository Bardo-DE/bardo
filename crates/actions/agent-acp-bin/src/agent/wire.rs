//! Building a host's wiring from this computer: the settings and `agents.toml` that allow an
//! agent to run, the sandbox, the bus and the launcher on it. `bardo-agent` on a terminal and
//! its service (`serve`) both start from here, so the two hold an agent to the same rules.

use super::fault::AgentError;
use super::host::Wiring;
use super::places::Places;
use super::sibling;
use action_dbus::BusConnection;
use action_dbus::callers::Identity;
use action_host::EditorDesk;
use action_settings::AgentSettings;
use agent_acp_host::ToolsOffer;
use agent_launch::dbus::DbusAccounts;
use agent_launch::{AgentsFile, ChildProc, Registry, Supervisor};
use sandbox::Detected;
use std::sync::Arc;

/// What `agents.toml` says, once `agent.acp.agents` is on and before anything starts: the programs
/// that may start.
#[derive(Debug)]
pub struct Allowed {
    /// The programs that may start.
    pub file: Arc<AgentsFile>,
}

/// Reads the setting and `agents.toml`. Off, or no file, or a file that is wrong, is a fault.
pub fn allowed(places: &Places) -> Result<Allowed, AgentError> {
    let loaded = places
        .locator()
        .read(AgentSettings::default(), action_settings::read);
    if !loaded.value.agents {
        return Err(AgentError::Permit(
            "agent.acp.agents is off: no agent program starts".to_owned(),
        ));
    }
    let text = std::fs::read_to_string(places.config_dir().join("actions/agents.toml"))
        .map_err(|e| AgentError::NoAgentsFile(std::sync::Arc::new(e)))?;
    let agents = places.agents_dir();
    let file = Arc::new(
        AgentsFile::parse_in(&text, Some(&agents))
            .map_err(|e| AgentError::Agents(e.to_string()))?,
    );
    Ok(Allowed { file })
}

/// The wiring of a host on `bus`: the sandbox, the launcher registered on the bus and the tool
/// edge (`tools`: whether the agent may be offered the desktop's actions, when `bardo-actions-mcp` is
/// beside this program; an entry in `agents.toml` can still switch it off for its program; a
/// start that only reads what the agent offers passes false). `fallback` says who answers the sheets of the agent's session; `places` is what
/// `main` read of the environment.
pub async fn build(
    allowed: Allowed,
    tools: bool,
    terminal_fallback: bool,
    bus: BusConnection,
    limits: SandboxLimits,
    places: Places,
) -> Result<Wiring<DbusAccounts>, AgentError> {
    let Allowed { file } = allowed;
    let bwrap = match limits.probe(places.path()) {
        Detected::Bwrap(bwrap) => bwrap,
        Detected::Missing(why) => return Err(AgentError::NoSandbox(why)),
    };
    let forwarder = sibling("bardo-agent-net-forward").ok_or(AgentError::NoForwarder)?;
    // Without the exec stage the agent is held by the sandbox alone: say so, never hide it.
    let enter =
        sibling("bardo-agent-net-enter").filter(|p| std::path::Path::new(p.as_str()).exists());
    if enter.is_none() {
        eprintln!(
            "bardo-agent: no bardo-agent-net-enter beside this program: the kernel does not limit the agent's files or network"
        );
    }
    let run_dir = places
        .runtime_dir()
        .ok_or(AgentError::NoRuntimeDir)?
        .to_path_buf();
    // The desktop's actions go to the agent as an MCP server it starts itself: the `bardo-actions-mcp`
    // program beside this one, as the bridge to this host. Whether a program is offered it (an
    // entry can switch it off) and which of the actions is decided where its backend is made.
    let offered = tools
        .then(|| sibling("bardo-actions-mcp"))
        .flatten()
        .filter(|bridge| std::path::Path::new(bridge.as_str()).exists())
        .map(|bridge| ToolsOffer {
            run_dir: run_dir.clone(),
            bridge,
        });
    if offered.is_none() && tools {
        eprintln!(
            "bardo-agent: no agent is offered the desktop's actions (no bardo-actions-mcp beside this program)"
        );
    }

    let table = crate::callers::read(&|name: &str| places.var(name))
        .map_err(|e| AgentError::Callers(e.to_string()))?;
    let callers = Arc::new(Identity::on_system(bus.clone(), table));
    let accounts = Arc::new(
        DbusAccounts::connect(&bus)
            .await
            .map_err(|e| AgentError::Bus(e.to_string()))?,
    );
    let registry: Registry<ChildProc> = Registry::default();
    let supervisor = Supervisor::new(
        Arc::clone(&accounts),
        Arc::clone(&file),
        registry.clone(),
        places.login_runner(),
    );
    supervisor
        .register()
        .await
        .map_err(|e| AgentError::Bus(e.to_string()))?;
    tokio::spawn(async move { supervisor.run().await });

    if places.data_dir().join("acp-agent-grants.json").exists() {
        eprintln!(
            "bardo-agent: ignoring the old acp-agent-grants.json in the data directory: \
             an agent's grants are actions's now (Settings lists and revokes them); allow \
             \"always\" again where you want one"
        );
    }
    Ok(Wiring {
        bus,
        accounts,
        file,
        bwrap: Detected::Bwrap(bwrap),
        forwarder,
        enter,
        run_dir,
        registry,
        tools: offered,
        terminal_fallback,
        desk: EditorDesk::new(),
        callers,
        agents: Some(places.agents_dir()),
        places,
    })
}

/// How the sandbox bounds what an agent uses.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum SandboxLimits {
    /// A scope of the person's systemd user manager holds the agent's memory and tasks: what the
    /// packaged program always uses.
    #[default]
    Scope,
    /// Per-process limits instead, for a machine with no user manager. Only the acceptance
    /// harness passes it (its jail and its live worlds run no systemd).
    #[cfg(feature = "testing")]
    PerProcess,
}

impl SandboxLimits {
    /// The sandbox here, bounded this way.
    #[must_use]
    pub fn probe(self, path: &str) -> Detected {
        match self {
            SandboxLimits::Scope => Detected::probe(path),
            #[cfg(feature = "testing")]
            SandboxLimits::PerProcess => {
                let hardening = sandbox_core::Hardening {
                    limits: sandbox_core::LimitPolicy::Rlimit(sandbox_core::Limits::default()),
                    ..sandbox_core::Hardening::default()
                };
                Detected::probe_with(path, &hardening, &sandbox::UserSession::from_env())
            }
        }
    }
}
