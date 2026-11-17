//! Which rows of the account modules are read with their value.

use settings_types::schema::KeyKind;

/// The rows whose value is read with the rows: the services (on or off) and the client ids.
#[must_use]
pub fn accounts_row_is_valued(kind: &KeyKind) -> bool {
    matches!(kind, KeyKind::Toggle | KeyKind::Text)
}
