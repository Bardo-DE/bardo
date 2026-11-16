//! Starting an agent installed from the registry: read the record `bardo-agents` left in the
//! agents directory and make the agent's home. The values are `agent_registry`'s; this is the only
//! disk access the launcher needs, so it does not link the installer.

use agent_registry::dirs::AgentsDir;
use agent_registry::launch::{Launch, LaunchFault};
use agent_registry::record::{FILE, Record, RecordFault};
use agent_registry::slug::Slug;
use std::sync::Arc;

/// How to start `id` at `version`; makes its home when it is not there yet.
pub fn launch(dir: &AgentsDir, id: &str, version: &str) -> Result<Launch, LaunchFault> {
    let (slug, pinned) = (Slug::parse(id)?, Slug::parse(version)?);
    let at = dir.installed(&slug, &pinned);
    let record = std::fs::read_to_string(at.join(FILE))
        .map_err(|error| RecordFault::Missing(Arc::new(error)))
        .and_then(|text| Record::parse(&text))
        .map_err(|why| LaunchFault::Record {
            id: id.to_owned(),
            version: version.to_owned(),
            why,
        })?;
    let home = dir.home(&slug);
    std::fs::create_dir_all(&home).map_err(|error| LaunchFault::Home(Arc::new(error)))?;
    Ok(Launch::of(&record, &at, home))
}
