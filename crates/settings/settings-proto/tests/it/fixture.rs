//! A small accounts-like schema the tests check.

use std::collections::BTreeMap;

use settings_proto::LiveSchema;
use settings_types::schema::{
    ActionLabel, ActionWeight, AgentSetting, Exposure, Help, KeyKind, KeyPath, KeySpec, Label,
    LiveAction, Page, Section,
};

pub const TOGGLE: &str = "accounts.a1.service.mail";
pub const CLIENT: &str = "accounts.clients.google";
pub const REMOVE: &str = "accounts.a1.remove";

pub fn spec(path: &str, kind: KeyKind, default: toml::Value) -> KeySpec {
    KeySpec {
        ends: None,
        path: KeyPath(path.to_owned()),
        kind,
        default,
        label: Label(path.to_owned()),
        help: Help(String::new()),
        page: Page::Accounts,
        section: Section("Accounts".to_owned()),
        exposure: Exposure::Basic,
        labels: settings_types::schema::WordLabels::default(),
        agent: AgentSetting::HandsOff,
        unavailable: BTreeMap::default(),
        groups: BTreeMap::default(),
        places: BTreeMap::default(),
        note: None,
        control: None,
    }
}

pub fn schema() -> LiveSchema {
    LiveSchema {
        version: 1,
        key: vec![
            spec(TOGGLE, KeyKind::Toggle, toml::Value::Boolean(true)),
            spec(CLIENT, KeyKind::Text, toml::Value::String(String::new())),
            spec(
                REMOVE,
                KeyKind::Live {
                    action: LiveAction {
                        label: ActionLabel("Remove account".to_owned()),
                        weight: ActionWeight::Destructive,
                    },
                },
                toml::Value::Boolean(false),
            ),
        ],
    }
}
