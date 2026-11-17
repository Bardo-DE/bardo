//! The coding agents the assistant may be handed to, in the actions service's files: whether they are switched
//! on (`agent.acp.agents` in the actions service's settings file), turning them on, and the known programs
//! that are already on this computer. Nothing is started: a program is looked for as a file.

use crate::error::FilesError;
use crate::read::text_of;
use crate::save::save;
use crate::values::ConfigHome;
use action_settings::{AgentSettings, Found, SETTINGS_FILE, dirs_of, find_known, read};
use settings_app_model::ConfigFile;
use settings_types::schema::{FilePath, KeyPath};
use std::path::Path;

/// The key that switches coding agents on.
const SWITCH: &str = "agent.acp.agents";

fn file() -> Result<ConfigFile, FilesError> {
    ConfigFile::parse(&FilePath(SETTINGS_FILE.to_owned())).map_err(|reason| FilesError::Garbled {
        path: SETTINGS_FILE.into(),
        reason: format!("{reason:?}"),
    })
}

/// Whether coding agents are switched on; off when there is no file, or it cannot be read.
#[must_use]
pub fn agents_switch(home: &ConfigHome) -> bool {
    let text = file()
        .ok()
        .and_then(|file| text_of(&home.path_of(&file)).ok().flatten())
        .unwrap_or_default();
    read(&text, AgentSettings::default()).value.agents
}

/// Switches coding agents on, leaving every other key of the file as it was.
///
/// # Errors
///
/// Returns a [`FilesError`] when the settings file cannot be read, is not valid TOML, or cannot be
/// written.
pub fn turn_on_agents(home: &ConfigHome) -> Result<(), FilesError> {
    save(
        home,
        &file()?,
        &KeyPath(SWITCH.to_owned()),
        &toml::Value::Boolean(true),
    )
}

/// The known coding agent programs that are files in the directories of `path` (the value of
/// `PATH`).
pub fn programs_on(path: &str) -> Vec<Found> {
    find_known(&dirs_of(path), Path::is_file)
}
