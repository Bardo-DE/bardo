//! Reading rows when the module moves between the listing and the values.

use super::read::{Rows, read};
use std::cell::Cell;
use std::collections::BTreeMap;
use ui_settings::live::LiveError;
use ui_settings::schema::{
    AgentSetting, Exposure, Help, KeyKind, KeyPath, KeySpec, Label, Page, Section, WordLabels,
};

fn row(name: &str) -> KeySpec {
    KeySpec {
        ends: None,
        control: None,
        path: KeyPath(name.to_owned()),
        kind: KeyKind::Text,
        default: toml::Value::String(String::new()),
        label: Label(name.to_owned()),
        help: Help(String::new()),
        page: Page::Accounts,
        section: Section("Account".to_owned()),
        exposure: Exposure::Basic,
        labels: WordLabels::default(),
        unavailable: BTreeMap::default(),
        groups: BTreeMap::default(),
        places: BTreeMap::default(),
        note: None,
        agent: AgentSetting::HandsOff,
    }
}

/// Two rows; the first `unknown` gets answer `UnknownKey`, as if the account just went.
struct Moving {
    unknown: Cell<usize>,
}

impl Rows for Moving {
    #[expect(
        clippy::unused_async_trait_impl,
        reason = "Rows is async; this fake answers at once"
    )]
    async fn describe(&self) -> Result<Vec<KeySpec>, LiveError> {
        Ok(vec![row("accounts.a.client"), row("accounts.b.client")])
    }

    #[expect(
        clippy::unused_async_trait_impl,
        reason = "Rows is async; this fake answers at once"
    )]
    async fn get(&self, path: &KeyPath) -> Result<toml::Value, LiveError> {
        match self.unknown.get() {
            0 => Ok(toml::Value::String(format!("value of {}", path.0))),
            left => {
                self.unknown.set(left - 1);
                Err(LiveError::UnknownKey(path.0.clone()))
            }
        }
    }
}

fn only_text(kind: &KeyKind) -> bool {
    matches!(kind, KeyKind::Text)
}

#[tokio::test]
async fn a_row_that_moves_once_is_read_over_and_the_list_is_whole() {
    let read = read(
        &Moving {
            unknown: Cell::new(1),
        },
        only_text,
    )
    .await
    .unwrap();
    assert_eq!(read.keys.len(), 2);
    assert_eq!(read.values.len(), 2);
}

#[tokio::test]
async fn a_module_that_keeps_moving_reports_the_error() {
    let read = read(
        &Moving {
            unknown: Cell::new(100),
        },
        only_text,
    )
    .await;
    read.unwrap_err();
}
