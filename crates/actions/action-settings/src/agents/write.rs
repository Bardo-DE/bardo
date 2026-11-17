//! Writes the person's choices into `agents.toml`: the model, the way of signing in and who keeps
//! the history of the files of one entry, and nothing else. The file is edited in place, so comments and every other field stay.

use action_core::Rewind;
use toml_edit::{DocumentMut, Item, value};

/// What to do with one field of the entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Pick {
    /// Leave it as it is.
    Keep,
    /// Set it to this id.
    Set(String),
    /// Remove it: the agent chooses.
    Clear,
}

/// The choices for one agent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentChoice {
    /// The model, by the id the agent gives it.
    pub model: Pick,
    /// The way of signing in, by the id the agent gives it.
    pub way: Pick,
}

/// Why the choice was not written.
#[derive(Debug, thiserror::Error)]
pub enum ChoiceError {
    /// `agents.toml` is not valid TOML.
    #[error("the agents file isn't valid TOML")]
    Syntax(#[source] toml_edit::TomlError),
    /// `agent` in `agents.toml` is not a list of entries.
    #[error("the agents file can't be read")]
    Unreadable,
    /// No such agent is listed.
    #[error("that agent isn't listed")]
    NoSuchAgent,
    /// The model id has spaces or is too long.
    #[error("that isn't a model name the actions service can keep")]
    BadModel,
    /// The sign-in id has odd characters or is too long.
    #[error("that isn't a way of signing in the actions service can keep")]
    BadWay,
    /// The file could not be read or written.
    #[error("the agents file can't be read or written")]
    Io(#[source] std::io::Error),
}

fn model_fine(id: &str) -> bool {
    !id.is_empty()
        && id.chars().count() <= 128
        && !id.chars().any(|c| c.is_whitespace() || c.is_control())
}

fn way_fine(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= 64
        && id
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.'))
}

fn apply(entry: &mut toml_edit::Table, key: &str, pick: &Pick) {
    match pick {
        Pick::Keep => {}
        Pick::Set(id) => {
            entry.insert(key, value(id.as_str()));
        }
        Pick::Clear => {
            entry.remove(key);
        }
    }
}

/// `text` with the choices set in the entry listed as `program`. Pure.
///
/// # Errors
///
/// [`ChoiceError`] when `text` is not readable TOML, no such agent is listed, or the model or the
/// way of signing in is not one actions service can keep.
pub fn edit_agent_choice(
    text: &str,
    program: &str,
    choice: &AgentChoice,
) -> Result<String, ChoiceError> {
    if matches!(&choice.model, Pick::Set(id) if !model_fine(id)) {
        return Err(ChoiceError::BadModel);
    }
    if matches!(&choice.way, Pick::Set(id) if !way_fine(id)) {
        return Err(ChoiceError::BadWay);
    }
    let mut doc: DocumentMut = text.parse().map_err(ChoiceError::Syntax)?;
    let entry = entry_of(&mut doc, program)?;
    apply(entry, "model", &choice.model);
    apply(entry, "sign_in", &choice.way);
    Ok(doc.to_string())
}

fn entry_of<'a>(
    doc: &'a mut DocumentMut,
    program: &str,
) -> Result<&'a mut toml_edit::Table, ChoiceError> {
    doc.get_mut("agent")
        .and_then(Item::as_array_of_tables_mut)
        .and_then(|list| {
            list.iter_mut()
                .find(|t| t.get("program").and_then(Item::as_str) == Some(program))
        })
        .ok_or(ChoiceError::NoSuchAgent)
}

/// `text` with `checkpoints` (who keeps the history of the files the agent changes) set in the
/// entry listed as `program`.
///
/// Written out in full, so a later change of the default does not move a choice the person made.
/// Pure.
///
/// # Errors
///
/// [`ChoiceError`] when `text` is not readable TOML or no such agent is listed.
pub fn edit_agent_rewind(text: &str, program: &str, rewind: Rewind) -> Result<String, ChoiceError> {
    let mut doc: DocumentMut = text.parse().map_err(ChoiceError::Syntax)?;
    let entry = entry_of(&mut doc, program)?;
    let word = match rewind {
        Rewind::Actions => "actions",
        Rewind::Agent => "agent",
    };
    entry.insert("checkpoints", value(word));
    Ok(doc.to_string())
}
