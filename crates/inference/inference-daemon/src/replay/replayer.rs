//! A loaded cassette and what has been answered from it.
//!
//! One [`Replayer`] lives as long as the daemon, not as long as an engine's process: an engine that
//! unloads when idle and is started again goes on from where the script was, it does not play the
//! `Once` entries a second time.

use super::cassette::{Cassette, Reply, Seen};
use model_replay::CassetteError;
use serde_json::Value;
use std::path::Path;
use std::sync::{Arc, Mutex, PoisonError};

/// Why a replay engine cannot answer. Every case is data: the engine turns it into an HTTP error
/// body or refuses to start, it never panics and never reaches for the network.
#[derive(Debug, Clone, thiserror::Error)]
pub enum ReplayError {
    /// The file could not be read.
    #[error("replay cassette {path} could not be read")]
    Unreadable {
        /// The path `inferd.toml` named.
        path: String,
        /// What the operating system said (shared: the error is cloned to each user of the
        /// engine).
        #[source]
        source: Arc<std::io::Error>,
    },
    /// The file is not a cassette.
    #[error("replay cassette {path} is not a cassette")]
    Malformed {
        /// The path `inferd.toml` named.
        path: String,
        /// What is wrong with it (shared, as above).
        #[source]
        why: Arc<CassetteError>,
    },
    /// No entry admits the request, or every one that does has been used.
    #[error("replay: no entry answers this request ({answered} of {entries} entries used)")]
    NoEntry {
        /// How many entries the cassette has.
        entries: u32,
        /// How many `Once` entries are spent.
        answered: u32,
    },
}

impl ReplayError {
    /// The slug the engine's error body carries as its `type`.
    #[must_use]
    pub fn slug(&self) -> &'static str {
        match self {
            ReplayError::Unreadable { .. } => "replay_unreadable",
            ReplayError::Malformed { .. } => "replay_malformed",
            ReplayError::NoEntry { .. } => "replay_miss",
        }
    }
}

/// A cassette and its cursor.
#[derive(Debug)]
pub struct Replayer {
    cassette: Cassette,
    used: Mutex<Vec<usize>>,
}

impl Replayer {
    /// Replays `cassette`.
    #[must_use]
    pub fn new(cassette: Cassette) -> Self {
        Self {
            cassette,
            used: Mutex::new(Vec::new()),
        }
    }

    /// Reads the cassette at `path` (the one `inferd.toml` names; nothing looks elsewhere).
    ///
    /// # Errors
    ///
    /// Returns a [`ReplayError`] when the file cannot be read or is not a cassette.
    pub fn load(path: &Path) -> Result<Self, ReplayError> {
        let shown = path.display().to_string();
        let text = std::fs::read_to_string(path).map_err(|e| ReplayError::Unreadable {
            path: shown.clone(),
            source: Arc::new(e),
        })?;
        Cassette::parse(&text)
            .map(Self::new)
            .map_err(|why| ReplayError::Malformed {
                path: shown,
                why: Arc::new(why),
            })
    }

    /// The cassette's header and entries.
    pub fn cassette(&self) -> &Cassette {
        &self.cassette
    }

    /// The reply for a chat request body, marking its entry used.
    ///
    /// # Errors
    ///
    /// Returns a [`ReplayError`] when no unused entry answers the body.
    pub fn answer(&self, body: &Value) -> Result<Reply, ReplayError> {
        let mut used = self.used.lock().unwrap_or_else(PoisonError::into_inner);
        let at =
            self.cassette
                .pick(&Seen::of(body), &used)
                .ok_or_else(|| ReplayError::NoEntry {
                    entries: u32::try_from(self.cassette.entries.len()).unwrap_or(u32::MAX),
                    answered: u32::try_from(used.len()).unwrap_or(u32::MAX),
                })?;
        used.push(at);
        Ok(self.cassette.entries[at].reply.clone())
    }
}

#[cfg(test)]
mod tests;
