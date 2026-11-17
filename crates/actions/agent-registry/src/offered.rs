//! What an agent last offered, written by the actions service whenever a session with it opens (and when its
//! models are listed), kept at `state/<id>/offered.toml` for the Settings app to read.
//!
//! This is the record as a value and as text; reading and writing the file is up to the caller.
//!
//! ```toml
//! seen = 3                    # how many times this was written: a counter, no clock
//! start = "signed_in"         # signed_in | needs_sign_in | unknown
//! in_use = "sonnet"           # the model the session ran, when the agent says
//! [[model]]
//! id = "sonnet"
//! name = "Sonnet 5.5"
//! [[way]]                     # the ways of signing in the agent listed
//! id = "login"
//! name = "Sign in with your account"
//! ```
//!
//! The record is the agent's own words, shown to the person and trusted for nothing: the ids the
//! person picks are checked again by the agent when the next session opens.

use crate::slug::SlugRefused;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// The file name of the record, inside the agent's state directory.
pub const FILE: &str = "offered.toml";

/// A thing the agent lists: its id as `agents.toml` writes it, and the name a person reads.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Named {
    /// The id.
    pub id: String,
    /// The agent's name for it.
    pub name: String,
}

/// How the last start went, as far as sign-in goes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Start {
    /// The session opened.
    SignedIn,
    /// The agent asked for a sign-in first; the ways it listed are in `ways`.
    NeedsSignIn,
    /// The start did not get as far as sign-in.
    Unknown,
}

/// What the agent offered at its last start.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Offered {
    /// How many times the record was written (a counter; the record never reads a clock).
    pub seen: u64,
    /// How the last start went.
    pub start: Start,
    /// The model the session ran, when the agent says.
    #[serde(default)]
    pub in_use: Option<String>,
    /// The models on offer; none when the agent offers no choice.
    #[serde(default, rename = "model")]
    pub models: Vec<Named>,
    /// The ways of signing in the agent listed.
    #[serde(default, rename = "way")]
    pub ways: Vec<Named>,
}

/// Why a record was not read or written. Two faults are equal when they are the same kind of
/// failure.
#[derive(Debug, Clone, thiserror::Error)]
pub enum OfferedFault {
    /// The id is not a name the actions service keeps agents under.
    #[error("the agent's name is not one actions service keeps records under")]
    Name(#[source] SlugRefused),
    /// The file could not be read or written.
    #[error("the record could not be read or written")]
    Io(#[source] Arc<std::io::Error>),
    /// The file is not a record.
    #[error("the record is damaged")]
    Damaged(#[source] toml::de::Error),
    /// The record cannot be written as TOML.
    #[error("the record cannot be written as TOML")]
    Unwritable(#[source] toml::ser::Error),
}

impl PartialEq for OfferedFault {
    fn eq(&self, other: &Self) -> bool {
        std::mem::discriminant(self) == std::mem::discriminant(other)
    }
}

impl Eq for OfferedFault {}

impl Offered {
    /// Reads the text of `offered.toml`.
    ///
    /// # Errors
    ///
    /// Returns [`OfferedFault::Damaged`] when `text` is not a valid record.
    pub fn parse(text: &str) -> Result<Self, OfferedFault> {
        toml::from_str(text).map_err(OfferedFault::Damaged)
    }

    /// The record as the text kept in `offered.toml`.
    ///
    /// # Errors
    ///
    /// Returns [`OfferedFault::Unwritable`] when the record cannot be written as TOML.
    pub fn render(&self) -> Result<String, OfferedFault> {
        toml::to_string(self).map_err(OfferedFault::Unwritable)
    }

    /// The record that replaces `before` when `self` is written: its counter one past the one it
    /// replaces. A start that did not get as far as listing models keeps the models listed before.
    #[must_use]
    pub fn after(self, before: Option<&Self>) -> Self {
        let seen = before.map_or(0, |o| o.seen) + 1;
        let (in_use, models) = match (self.models.is_empty(), before) {
            (true, Some(old)) if self.start != Start::SignedIn => {
                (old.in_use.clone(), old.models.clone())
            }
            _ => (self.in_use.clone(), self.models.clone()),
        };
        Self {
            seen,
            in_use,
            models,
            ..self
        }
    }
}
