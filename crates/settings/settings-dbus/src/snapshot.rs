//! Reading a module's snapshot from its schema and one `Get` per key.

use settings_proto::{LiveSchema, Snapshot};
use settings_types::schema::KeyPath;

use super::LiveError;

/// The snapshot of `schema` given what a `Get` answered for each of its keys.
///
/// A key that answered [`LiveError::UnknownKey`] went away since the schema was read: it is
/// dropped from the schema as well, so the two agree. Any other failure is the read's failure.
/// A key in `reads` that the schema does not list is ignored.
///
/// # Errors
///
/// Returns the first [`LiveError`] a read gave, other than [`LiveError::UnknownKey`].
pub fn snapshot_from_reads(
    schema: LiveSchema,
    reads: impl IntoIterator<Item = (KeyPath, Result<toml::Value, LiveError>)>,
) -> Result<Snapshot, LiveError> {
    let mut values = std::collections::BTreeMap::new();
    for (key, read) in reads {
        match read {
            Ok(value) => {
                values.insert(key, value);
            }
            Err(LiveError::UnknownKey(_)) => {}
            Err(other) => return Err(other),
        }
    }
    Ok(Snapshot::agreeing(schema, values))
}
