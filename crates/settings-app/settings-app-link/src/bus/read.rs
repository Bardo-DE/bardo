//! Reading a module's rows and their values, as one reading even when the module moves meanwhile.

use super::live_absence;
use settings_app_model::{Absence, Described};
use settings_dbus::{LiveClient, LiveError};
use settings_types::schema::{KeyKind, KeyPath, KeySpec};
use std::collections::BTreeMap;

/// How many times a reading is made over when the module's rows move under it.
const ATTEMPTS: usize = 4;

/// What a reading asks of a module: its rows, and a row's value.
pub(crate) trait Rows {
    async fn describe(&self) -> Result<Vec<KeySpec>, LiveError>;
    async fn get(&self, path: &KeyPath) -> Result<toml::Value, LiveError>;
}

impl Rows for LiveClient {
    async fn describe(&self) -> Result<Vec<KeySpec>, LiveError> {
        Ok(LiveClient::describe(self).await?.key)
    }

    async fn get(&self, path: &KeyPath) -> Result<toml::Value, LiveError> {
        LiveClient::get(self, path).await
    }
}

/// The rows of the live client `client` as they stand now, a value for each row `valued` picks
/// out.
///
/// # Errors
///
/// The `Absence` the module's failure maps to, when its rows keep moving or a call fails.
pub async fn read_live(
    client: &LiveClient,
    valued: fn(&KeyKind) -> bool,
) -> Result<Described, Absence> {
    read(client, valued).await
}

/// The module's rows and the value of each row `valued` picks out. A row that was listed and is
/// unknown when asked for means the rows moved, so the whole reading is made again.
pub(crate) async fn read(
    module: &impl Rows,
    valued: fn(&KeyKind) -> bool,
) -> Result<Described, Absence> {
    for _ in 1..ATTEMPTS {
        match pass(module, valued).await {
            Err(LiveError::UnknownKey(_)) => {}
            done => return done.map_err(|error| live_absence(&error)),
        }
    }
    pass(module, valued)
        .await
        .map_err(|error| live_absence(&error))
}

async fn pass(module: &impl Rows, valued: fn(&KeyKind) -> bool) -> Result<Described, LiveError> {
    let keys = module.describe().await?;
    let mut scalars = BTreeMap::new();
    let mut word_lists = BTreeMap::new();
    for spec in keys.iter().filter(|spec| valued(&spec.kind)) {
        match module.get(&spec.path).await? {
            toml::Value::String(word) => {
                scalars.insert(spec.path.clone(), word);
            }
            toml::Value::Boolean(on) => {
                scalars.insert(spec.path.clone(), on.to_string());
            }
            toml::Value::Array(items) => {
                word_lists.insert(spec.path.clone(), words_of(&items));
            }
            _ => {}
        }
    }
    Ok(Described {
        keys,
        values: scalars,
        lists: word_lists,
    })
}

/// The words of a list value; an item that is not a word is left out.
#[must_use]
pub fn words_of(items: &[toml::Value]) -> Vec<String> {
    items
        .iter()
        .filter_map(|item| item.as_str().map(str::to_owned))
        .collect()
}
