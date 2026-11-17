//! Starting an agent installed from the registry: read the record `bardo-agents` left in the
//! agents directory and make the agent's home. The values are `agent_registry`'s; this is the only
//! disk access the launcher needs, so it does not link the installer.

use agent_registry::dirs::AgentsDir;
use agent_registry::launch::{Launch, LaunchError};
use agent_registry::record::{FILE, Record, RecordError};
use agent_registry::slug::Slug;
use std::sync::Arc;

/// How to start `id` at `version`; makes its home when it is not there yet.
///
/// # Errors
///
/// Returns the [`LaunchError`] (boxed, as it is large) when `id` or `version` is not a name agents
/// are kept under, the record is missing or damaged, or the home cannot be made.
pub fn launch(dir: &AgentsDir, id: &str, version: &str) -> Result<Launch, Box<LaunchError>> {
    let slug = Slug::parse(id).map_err(|e| Box::new(LaunchError::from(e)))?;
    let pinned = Slug::parse(version).map_err(|e| Box::new(LaunchError::from(e)))?;
    let at = dir.installed(&slug, &pinned);
    let record = std::fs::read_to_string(at.join(FILE))
        .map_err(|error| RecordError::Missing(Arc::new(error)))
        .and_then(|text| Record::parse(&text))
        .map_err(|why| {
            Box::new(LaunchError::Record {
                id: id.to_owned(),
                version: version.to_owned(),
                why,
            })
        })?;
    let home = dir.home(&slug);
    std::fs::create_dir_all(&home).map_err(|error| Box::new(LaunchError::Home(Arc::new(error))))?;
    Ok(Launch::of(&record, &at, home))
}
