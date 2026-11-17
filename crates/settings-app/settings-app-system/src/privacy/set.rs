//! The calls the page makes on the compositor's module.

use settings_app_link::bus::plain;
use settings_app_link::modules::Module;
use settings_app_model::{ALLOWED_WORD, DENIED_WORD};
use settings_types::schema::KeyPath;

/// Allows or denies at `path`; the module's own words when it refuses.
pub async fn set(client: &Module, path: &KeyPath, to: bool) -> Result<(), String> {
    client
        .set(
            path,
            &toml::Value::String(if to { ALLOWED_WORD } else { DENIED_WORD }.to_owned()),
        )
        .await
        .map_err(|error| plain(&error))
}

/// Asks again at `path`; the module's own words when it refuses.
pub async fn forget(client: &Module, path: &KeyPath) -> Result<(), String> {
    client.invoke(path).await.map_err(|error| plain(&error))
}
