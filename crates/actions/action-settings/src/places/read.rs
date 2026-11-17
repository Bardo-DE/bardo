//! The lenient reader of the places: anything it does not clearly understand is Off, never On.

use super::model::{CloudAccount, CloudAllowed, ModelChoice, OwnComputer, PlaceName, Places};
use settings_store::InvalidKey;

/// Where the places live in the settings file.
pub const PLACES_PATH: &str = "assistant";

/// The places read, and the rows that fell back.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlacesLoaded {
    /// The places in force.
    pub places: Places,
    /// Values refused (the row is then Off).
    pub invalid: Vec<InvalidKey>,
}

fn text(t: &toml::Table, key: &str) -> Option<String> {
    t.get(key)?.as_str().map(str::to_owned)
}

fn model(t: &toml::Table) -> Option<ModelChoice> {
    text(t, "model").filter(|m| !m.is_empty()).map(ModelChoice)
}

fn rows<'a>(doc: &'a toml::Table, key: &str) -> Vec<&'a toml::Table> {
    doc.get("assistant")
        .and_then(toml::Value::as_table)
        .and_then(|a| a.get(key))
        .and_then(toml::Value::as_array)
        .map(|rows| rows.iter().filter_map(toml::Value::as_table).collect())
        .unwrap_or_default()
}

fn word<T: Copy>(
    row: &toml::Table,
    key: &str,
    words: &'static [&'static str],
    values: &[T],
    off: T,
    invalid: &mut Vec<InvalidKey>,
) -> T {
    let found = row
        .get("allowed")
        .and_then(toml::Value::as_str)
        .and_then(|w| words.iter().position(|x| *x == w));
    found.map_or_else(
        || {
            invalid.push(InvalidKey {
                path: format!("{PLACES_PATH}.{key}.allowed"),
                reason: format!("not one of {}", words.join(", ")),
            });
            off
        },
        |i| values[i],
    )
}

/// Reads the places from settings text. Rows without a name are dropped; a missing or unknown
/// `allowed` is Off (and reported); "ask" on a computer of the person's is Off.
pub fn read_places(text_in: &str) -> PlacesLoaded {
    let Ok(doc) = text_in.parse::<toml::Table>() else {
        return PlacesLoaded {
            places: Places::none(),
            invalid: Vec::new(),
        };
    };
    let mut invalid = Vec::new();
    let this_computer = doc
        .get("assistant")
        .and_then(toml::Value::as_table)
        .and_then(|a| a.get("this_computer"))
        .and_then(toml::Value::as_table)
        .and_then(model);
    let own = rows(&doc, "own_computers")
        .into_iter()
        .filter_map(|row| {
            Some(OwnComputer {
                name: PlaceName(text(row, "name")?),
                allowed: word(
                    row,
                    "own_computers",
                    &["off", "on"],
                    &[false, true],
                    false,
                    &mut invalid,
                ),
                model: model(row),
            })
        })
        .collect();
    let cloud = rows(&doc, "cloud_accounts")
        .into_iter()
        .filter_map(|row| {
            Some(CloudAccount {
                name: PlaceName(text(row, "name")?),
                provider: text(row, "provider").unwrap_or_default(),
                allowed: word(
                    row,
                    "cloud_accounts",
                    &["off", "on", "ask"],
                    &[
                        CloudAllowed::Off,
                        CloudAllowed::On,
                        CloudAllowed::AskEachTime,
                    ],
                    CloudAllowed::Off,
                    &mut invalid,
                ),
                model: model(row),
            })
        })
        .collect();
    PlacesLoaded {
        places: Places {
            this_computer,
            own,
            cloud,
        },
        invalid,
    }
}
