//! The refusals for a reply over the limits (`settings_proto::limits`).

use settings_proto::{LiveSchema, limits};

use super::LiveError;

/// The refusal for a reply over a limit: plain words, no limit named.
fn too_big() -> LiveError {
    LiveError::Failed("The service sent more than a settings page can show.".to_owned())
}

/// `Ok` when `count` rows fit a page.
pub(super) fn rows(count: usize) -> Result<(), LiveError> {
    if limits::rows_fit(count) {
        Ok(())
    } else {
        Err(too_big())
    }
}

/// The schema `json` holds, checked as [`LiveSchema::from_json`] checks it and kept within the
/// limits.
pub(super) fn schema(json: &str) -> Result<LiveSchema, LiveError> {
    if json.len() > limits::SCHEMA_JSON_MAX {
        return Err(too_big());
    }
    let schema = LiveSchema::from_json(json).map_err(|why| LiveError::Failed(why.to_string()))?;
    if limits::schema_fits(&schema) {
        Ok(schema)
    } else {
        Err(too_big())
    }
}

/// `value` when it fits: a string or list a page can show.
pub(super) fn value(value: toml::Value) -> Result<toml::Value, LiveError> {
    if limits::value_fits(&value) {
        Ok(value)
    } else {
        Err(too_big())
    }
}
