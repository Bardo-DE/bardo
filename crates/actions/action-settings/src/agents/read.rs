//! Reads the rows from `agents.toml` text and the records beside the agents.

use super::model::{AgentRow, Agents, Availability, ModelState, Offers, SignInState, Source};
use action_core::Rewind;
use agent_registry::offered::{Offered, Start};
use agent_registry::slug::Slug;
use std::path::Path;

fn text(t: &toml::Table, key: &str) -> Option<String> {
    t.get(key)?
        .as_str()
        .filter(|s| !s.is_empty())
        .map(str::to_owned)
}

/// What the records beside the agents say. The disk is behind it; this crate reads none.
pub trait AgentRecords {
    /// Whether `version` of `id` is installed: its install record is there and is one.
    fn installed(&self, id: &Slug, version: &Slug) -> bool;

    /// What the agent installed as `id` last offered; none when it never started or the record
    /// is not readable.
    fn offered(&self, id: &Slug) -> Option<Offered>;

    /// Whether `path` is a file.
    fn is_file(&self, path: &Path) -> bool;
}

fn source(entry: &toml::Table, records: &impl AgentRecords) -> Source {
    let (Some(id), Some(version)) = (text(entry, "registry"), text(entry, "version")) else {
        // The program the person named is there if the file is: nothing is run to find out.
        let installed =
            text(entry, "command").is_some_and(|command| records.is_file(Path::new(&command)));
        return Source::Own { installed };
    };
    let installed = Slug::parse(&id)
        .ok()
        .zip(Slug::parse(&version).ok())
        .is_some_and(|(id, version)| records.installed(&id, &version));
    Source::Registry {
        id,
        version,
        installed,
    }
}

fn model(chosen: Option<String>, offers: &Offers) -> ModelState {
    let Some(id) = chosen else {
        return ModelState::AgentsOwn;
    };
    let availability = match offers {
        Offers::Seen { models, .. } if !models.is_empty() => models
            .iter()
            .find(|m| m.id == id)
            .map_or(Availability::NoLongerOffered, |m| {
                Availability::Offered(m.name.clone())
            }),
        _ => Availability::NotCheckedYet,
    };
    ModelState::Chosen { id, availability }
}

fn seen(record: Option<Offered>) -> (Offers, SignInState) {
    let Some(r) = record else {
        return (Offers::NotSeenYet, SignInState::NotSeenYet);
    };
    let sign_in = match r.start {
        Start::SignedIn => SignInState::SignedIn,
        Start::NeedsSignIn => SignInState::NeedsSignIn { ways: r.ways },
        Start::Unknown => SignInState::Unknown,
    };
    let offers = Offers::Seen {
        models: r.models,
        in_use: r.in_use,
    };
    (offers, sign_in)
}

/// What `checkpoints` says; unwritten (or not a word we know), the profile decides.
fn rewind(entry: &toml::Table) -> Rewind {
    match text(entry, "checkpoints").as_deref() {
        Some("actions") => Rewind::Actions,
        Some("agent") => Rewind::Agent,
        _ => match text(entry, "profile").as_deref() {
            Some("claude-code") => Rewind::Agent,
            _ => Rewind::Actions,
        },
    }
}

fn row(entry: &toml::Table, records: &impl AgentRecords) -> Option<AgentRow> {
    let program = text(entry, "program")?;
    let source = source(entry, records);
    let record = match &source {
        Source::Registry { id, .. } => Slug::parse(id).ok().and_then(|id| records.offered(&id)),
        Source::Own { .. } => None,
    };
    let (offers, sign_in) = seen(record);
    Some(AgentRow {
        command: text(entry, "command"),
        label: text(entry, "label").unwrap_or_else(|| program.clone()),
        model: model(text(entry, "model"), &offers),
        way: text(entry, "sign_in"),
        rewind: rewind(entry),
        program,
        source,
        offers,
        sign_in,
    })
}

/// The rows of `agents.toml` (its text), one per listed agent, against `records`.
///
/// Lenient: text that is not TOML gives no rows, and an entry with no name is left out. Starts
/// nothing.
pub fn read_agents(agents_toml: &str, records: &impl AgentRecords) -> Agents {
    let Ok(doc) = agents_toml.parse::<toml::Table>() else {
        return Agents::default();
    };
    let rows = doc
        .get("agent")
        .and_then(toml::Value::as_array)
        .map(|list| {
            list.iter()
                .filter_map(toml::Value::as_table)
                .filter_map(|entry| row(entry, records))
                .collect()
        })
        .unwrap_or_default();
    Agents { rows }
}
