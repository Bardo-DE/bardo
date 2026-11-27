//! The errors of the pure crate: a malformed id, a refused table and a refused caller.

use crate::ids::UnitName;
use crate::standing::WhyNot;
use std::io;

/// A text that is not the id it was offered as.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("{what} is not valid: {reason}")]
pub struct IdError {
    /// Which id was read (`"unit name"`).
    pub what: &'static str,
    /// Why it was refused, in a few words.
    pub reason: &'static str,
}

impl IdError {
    pub(crate) const fn new(what: &'static str, reason: &'static str) -> Self {
        Self { what, reason }
    }
}

/// What is wrong with one `[[peer]]` row.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum RowError {
    /// A field is not a valid id.
    #[error("{0}")]
    Id(#[from] IdError),
    /// The row lists no image path. A unit name alone proves nothing.
    #[error("it lists no `exe` path: a unit name alone proves nothing")]
    NoExe,
    /// The row's free-text name is empty or too long.
    #[error("its `name` is empty or longer than {max} bytes")]
    BadName {
        /// The longest accepted name.
        max: usize,
    },
}

/// A table text that was refused. A table is read whole or not at all.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum TableError {
    /// The text is not the table format (TOML with `[[peer]]` rows and no other keys).
    #[error("the table is not readable: {0}")]
    Syntax(String),
    /// The text is longer than any real table.
    #[error("the table is longer than {max} bytes")]
    TooLong {
        /// The longest accepted text.
        max: usize,
    },
    /// The table has more rows than any real one.
    #[error("the table has more than {max} rows")]
    TooManyRows {
        /// The most rows accepted.
        max: usize,
    },
    /// One row is wrong.
    #[error("row {index} ({app}): {fault}")]
    Row {
        /// The row's place in the file, from 1.
        index: usize,
        /// The row's `app`, as written.
        app: String,
        /// What is wrong with it.
        fault: RowError,
    },
    /// Two rows name the same unit.
    #[error("two rows name the unit {0}")]
    DuplicateUnit(UnitName),
}

/// A caller that does not hold what a method asks for. The texts are fixed on purpose: what the
/// caller learns must not say which check failed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum NotHeldError {
    /// The message carried no sender.
    #[error("no sender")]
    NoSender,
    /// The caller could not be identified.
    #[error("unknown caller")]
    Unknown,
    /// The caller is known and is not one this method answers.
    #[error("not the caller this method answers")]
    NotAllowed,
}

/// The facts about a process that could not be read.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum ProbeError {
    /// The process left before or while its facts were read.
    #[error("the process has exited")]
    PeerGone,
    /// A file could not be read.
    #[error("{what} could not be read")]
    Unreadable {
        /// Which fact.
        what: &'static str,
        /// The cause.
        #[source]
        source: io::Error,
    },
    /// A file is not UTF-8 text.
    #[error("{what} is not text")]
    NotText {
        /// Which fact.
        what: &'static str,
        /// The cause.
        #[source]
        source: std::string::FromUtf8Error,
    },
    /// A cgroup file does not name a valid cgroup.
    #[error("{what} is not a valid cgroup")]
    BadCgroup {
        /// Which fact.
        what: &'static str,
        /// The cause.
        #[source]
        source: IdError,
    },
    /// A file does not say what a file of its kind says, or is over a limit.
    #[error("{what} is malformed")]
    Malformed {
        /// Which fact.
        what: &'static str,
    },
}

impl ProbeError {
    /// Why a process that could not be probed is unknown: gone, or unreadable. A new variant
    /// fails as unreadable until it is given its own reason.
    #[must_use]
    pub fn why_not(&self) -> WhyNot {
        match self {
            ProbeError::PeerGone => WhyNot::PeerGone,
            _ => WhyNot::Unreadable,
        }
    }
}
