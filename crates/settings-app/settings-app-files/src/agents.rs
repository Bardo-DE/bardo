//! The outside agents the actions service may start (`actions/agents.toml`), read with what each last offered,
//! and the person's choice of model or way of signing in written back into the entry.

use crate::error::FilesError;
use crate::read::text_of;
use crate::values::{ConfigHome, DataHome};
use action_settings::{AddFault, AgentChoice, Agents, ChoiceFault, NewAgent};
use action_settings_fs::{read_agents_in, write_agent_add, write_agent_choice, write_agent_remove};
use agent_install::store::{agents, versions};
use agent_registry::dirs::AgentsDir;
use settings_app_model::FoundAgent;

/// The agents file, under the configuration directory.
const AGENTS_FILE: &str = "bardo/actions/agents.toml";

/// Where the actions service keeps what it installed and what each agent offered, under the data directory.
pub(crate) const AGENTS_DIR: &str = "bardo/actions/agents";

/// The agents `bardo/actions/agents.toml` lists, none when there is no file.
#[must_use]
pub fn load_agents(config: &ConfigHome, data: &DataHome) -> Agents {
    // A file that cannot be read lists no agents; the actions service's own writer refuses to write over it.
    let text = text_of(&config.0.join(AGENTS_FILE))
        .ok()
        .flatten()
        .unwrap_or_default();
    read_agents_in(&text, &AgentsDir::at(&data.0.join(AGENTS_DIR)))
}

/// Sets the person's `choice` in the entry of `program`; nothing else in the file changes.
///
/// # Errors
///
/// Returns a [`FilesError`] when `agents.toml` cannot be read, is not valid TOML, has no entry for
/// `program`, or cannot be written.
pub fn save_agent_choice(
    config: &ConfigHome,
    program: &str,
    choice: &AgentChoice,
) -> Result<(), FilesError> {
    write_agent_choice(&config.0.join(AGENTS_FILE), program, choice).map_err(|fault| match fault {
        ChoiceFault::Io(source) => FilesError::Io {
            path: AGENTS_FILE.into(),
            source,
        },
        _ => FilesError::Garbled {
            path: AGENTS_FILE.into(),
            reason: fault.to_string(),
        },
    })
}

/// The coding assistants installed in the actions service's folder, whether `agents.toml` lists them or not: each id with its last installed version, in name order.
///
/// Only files are read; nothing is started and no network is used.
#[must_use]
pub fn installed_agents(data: &DataHome) -> Vec<FoundAgent> {
    let dir = AgentsDir::at(&data.0.join(AGENTS_DIR));
    agents(&dir)
        .into_iter()
        .filter_map(|id| {
            let version = versions(&dir, &id).pop()?;
            Some(FoundAgent {
                id: id.as_str().to_owned(),
                version: version.as_str().to_owned(),
            })
        })
        .collect()
}

/// Lists `agent` in `agents.toml`, creating the file when there is none.
///
/// # Errors
///
/// Returns an [`AddFault`] when the entry is not valid, or is already listed, or the file cannot be
/// read or written.
pub fn add_agent(config: &ConfigHome, agent: &NewAgent) -> Result<(), AddFault> {
    write_agent_add(&config.0.join(AGENTS_FILE), agent)
}

/// Takes the entry of `program` out of `agents.toml`. Nothing else changes: the files installed
/// and the sign-in the assistant holds stay where they are.
///
/// # Errors
///
/// Returns a [`ChoiceFault`] when there is no entry for `program`, or the file cannot be read or
/// written.
pub fn remove_agent(config: &ConfigHome, program: &str) -> Result<(), ChoiceFault> {
    write_agent_remove(&config.0.join(AGENTS_FILE), program)
}
