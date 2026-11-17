//! The records beside the agents, read from the agents directory, and the writes of
//! `agents.toml`: the file is replaced whole or not at all.

use action_core::Rewind;
use action_settings::{
    AddFault, AgentChoice, AgentRecords, Agents, ChoiceError, NewAgent, edit_agent_add,
    edit_agent_choice, edit_agent_remove, edit_agent_rewind, read_agents,
};
use action_store::write_atomic;
use agent_registry::dirs::AgentsDir;
use agent_registry::offered::Offered;
use agent_registry::record::{FILE, Record};
use agent_registry::slug::Slug;
use std::path::Path;

/// The records of the agents in an agents directory, read from its files.
#[derive(Debug, Clone, Copy)]
pub struct DiskRecords<'a>(pub &'a AgentsDir);

impl AgentRecords for DiskRecords<'_> {
    fn installed(&self, id: &Slug, version: &Slug) -> bool {
        std::fs::read_to_string(self.0.installed(id, version).join(FILE))
            .is_ok_and(|text| Record::parse(&text).is_ok())
    }

    fn offered(&self, id: &Slug) -> Option<Offered> {
        let text = std::fs::read_to_string(self.0.offered(id)).ok()?;
        Offered::parse(&text).ok()
    }

    fn is_file(&self, path: &Path) -> bool {
        path.is_file()
    }
}

/// The rows of `agents.toml` (its text) against the records in `dir`.
///
/// Reads files; starts nothing. See [`action_settings::read_agents`].
#[must_use]
pub fn read_agents_in(agents_toml: &str, dir: &AgentsDir) -> Agents {
    read_agents(agents_toml, &DiskRecords(dir))
}

/// Adds `agent` to the file at `path`, which need not exist yet; the file is replaced whole or
/// not at all.
///
/// # Errors
///
/// [`AddError`] when the file cannot be read or written, or [`edit_agent_add`] refuses the edit.
pub fn write_agent_add(path: &Path, agent: &NewAgent) -> Result<(), AddError> {
    let text = match std::fs::read_to_string(path) {
        Ok(text) => text,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => String::new(),
        Err(error) => return Err(AddError::Io(error.into())),
    };
    let edited = edit_agent_add(&text, agent)?;
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).map_err(|error| AddError::Io(error.into()))?;
    }
    write_atomic(path, edited.as_bytes()).map_err(|error| AddError::Io(error.into()))
}

/// Takes the entry of `program` out of the file at `path`; the file is replaced whole or not at
/// all.
///
/// # Errors
///
/// [`ChoiceError`] when the file cannot be read or written, or [`edit_agent_remove`] refuses the
/// edit.
pub fn write_agent_remove(path: &Path, program: &str) -> Result<(), ChoiceError> {
    let text = std::fs::read_to_string(path).map_err(ChoiceError::Io)?;
    let edited = edit_agent_remove(&text, program)?;
    write_atomic(path, edited.as_bytes()).map_err(ChoiceError::Io)
}

/// Sets `checkpoints` in the entry of `program` in the file at `path`; the file is replaced
/// whole or not at all.
///
/// # Errors
///
/// [`ChoiceError`] when the file cannot be read or written, or [`edit_agent_rewind`] refuses the
/// edit.
pub fn write_agent_rewind(path: &Path, program: &str, rewind: Rewind) -> Result<(), ChoiceError> {
    let text = std::fs::read_to_string(path).map_err(ChoiceError::Io)?;
    let edited = edit_agent_rewind(&text, program, rewind)?;
    write_atomic(path, edited.as_bytes()).map_err(ChoiceError::Io)
}

/// Sets the choices in the entry of `program` in the file at `path`; the file is replaced whole
/// or not at all.
///
/// # Errors
///
/// [`ChoiceError`] when the file cannot be read or written, or [`edit_agent_choice`] refuses the
/// edit.
pub fn write_agent_choice(
    path: &Path,
    program: &str,
    choice: &AgentChoice,
) -> Result<(), ChoiceError> {
    let text = std::fs::read_to_string(path).map_err(ChoiceError::Io)?;
    let edited = edit_agent_choice(&text, program, choice)?;
    write_atomic(path, edited.as_bytes()).map_err(ChoiceError::Io)
}
