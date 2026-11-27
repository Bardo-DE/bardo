//! What `Describe` returns: a live module's keys, in the same [`KeySpec`] shape as a file schema.

use serde::{Deserialize, Serialize};

use settings_types::schema::{KeyPath, KeySpec, never_settable_violation};

/// A live module's schema. JSON on the wire: `{"version":1,"key":[ ...KeySpec... ]}`.
///
/// Unlike a file [`Schema`](settings_types::schema::Schema) it names no app or file (the bus name and
/// object path say who serves it), and its keys may use [`KeyKind::Live`](settings_types::schema::KeyKind::Live).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LiveSchema {
    /// The schema format's version.
    pub version: u16,
    /// The module's keys, in the order a page shows them.
    pub key: Vec<KeySpec>,
}

/// Why a [`LiveSchema`] is refused.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum LiveSchemaError {
    /// The text is not JSON of the live schema shape.
    #[error("the schema is not valid JSON of the live schema shape: {0}")]
    Parse(String),
    /// Section 9.7: an `accounts.`, `ai.`, ... key can never be marked agent-settable.
    #[error("key {} is under a never-agent-settable prefix but is marked agent = \"settable\"", .0.0)]
    NeverSettable(KeyPath),
    /// A key path is listed twice.
    #[error("key {} appears twice", .0.0)]
    Duplicate(KeyPath),
}

impl LiveSchema {
    /// The JSON `Describe` returns.
    ///
    /// # Panics
    ///
    /// Panics only if the schema cannot be written as JSON, which its field types rule out.
    #[must_use]
    pub fn to_json(&self) -> String {
        serde_json::to_string(self)
            .expect("LiveSchema fields are all JSON-representable: strings, numbers, enums, Vec")
    }

    /// The schema `text` holds, checked ([`LiveSchema::check`]).
    ///
    /// # Errors
    ///
    /// Returns [`LiveSchemaError::Parse`] when `text` is not a schema, and the errors of
    /// [`LiveSchema::check`].
    pub fn from_json(text: &str) -> Result<LiveSchema, LiveSchemaError> {
        let schema: LiveSchema =
            serde_json::from_str(text).map_err(|e| LiveSchemaError::Parse(e.to_string()))?;
        schema.check().map(|()| schema)
    }

    /// The same never-settable check a file schema gets (`Schema::from_toml`, through
    /// [`never_settable_violation`]), plus: no key twice.
    ///
    /// # Errors
    ///
    /// Returns [`LiveSchemaError::NeverSettable`] when a key under a never-settable prefix is
    /// marked agent-settable, and [`LiveSchemaError::Duplicate`] when a key appears twice.
    pub fn check(&self) -> Result<(), LiveSchemaError> {
        if let Some(key) = never_settable_violation(&self.key) {
            return Err(LiveSchemaError::NeverSettable(key.path.clone()));
        }
        let mut seen = std::collections::HashSet::new();
        match self.key.iter().find(|key| !seen.insert(&key.path)) {
            Some(key) => Err(LiveSchemaError::Duplicate(key.path.clone())),
            None => Ok(()),
        }
    }
}
