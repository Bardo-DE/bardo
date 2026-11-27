//! A module read once: its schema and every key's value, agreeing with each other.

use std::collections::BTreeMap;

use crate::LiveSchema;
use settings_types::schema::KeyPath;

/// What `Snapshot` returns: a module's [`LiveSchema`] and the value of every key in it.
///
/// Every key in `values` is in `schema` and every key in `schema` has a value, so a reader never
/// sees a half-changed module (a `Describe` that lists an account and a `Get` that no longer
/// knows it). A module that can read its state in one go answers from one lock (accountd does);
/// the default reads `Describe` then each key.
#[derive(Debug, Clone, PartialEq)]
pub struct Snapshot {
    /// The module's schema.
    pub schema: LiveSchema,
    /// The value of every key in `schema`.
    pub values: BTreeMap<KeyPath, toml::Value>,
}

impl Snapshot {
    /// The snapshot of `schema` and the `values` read for its keys, made to agree.
    ///
    /// A key in `schema` with no value went away since the schema was read: it is dropped from the
    /// schema. A value whose key the schema does not list is dropped too.
    #[must_use]
    pub fn agreeing(
        mut schema: LiveSchema,
        mut values: BTreeMap<KeyPath, toml::Value>,
    ) -> Snapshot {
        schema.key.retain(|spec| values.contains_key(&spec.path));
        values.retain(|key, _| schema.key.iter().any(|spec| spec.path == *key));
        Snapshot { schema, values }
    }
}
