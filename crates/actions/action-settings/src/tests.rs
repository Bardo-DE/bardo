//! The schema, the key table and the reader hold to one another.

use super::*;
use crate::keys::table;
use settings_store::rules::Rule;
use settings_store::{InvalidKey, UnknownKey};
use std::collections::BTreeSet;

/// A spend cap of `n` millionths of a US dollar.
fn micro_usd(n: i64) -> model_types::Money {
    model_types::Money::usd(n, 6)
}

fn schema() -> toml::Table {
    SCHEMA.parse().expect("the schema is TOML")
}

fn rows() -> Vec<toml::Table> {
    schema()["key"]
        .as_array()
        .expect("key tables")
        .iter()
        .map(|k| k.as_table().expect("a table").clone())
        .collect()
}

fn path(row: &toml::Table) -> &str {
    row["path"].as_str().expect("path")
}

/// A kind with no parameters, such as a toggle, has no `v` table.
static NO_PARAMS: std::sync::LazyLock<toml::Table> = std::sync::LazyLock::new(toml::Table::new);

fn kind_of(row: &toml::Table) -> (&str, &toml::Table) {
    let kind = row["kind"].as_table().expect("kind");
    (
        kind["kind"].as_str().expect("kind name"),
        kind.get("v")
            .and_then(toml::Value::as_table)
            .unwrap_or(&NO_PARAMS),
    )
}

/// `text` for one key set to `value`, as a nested table document.
fn doc(path: &str, value: toml::Value) -> String {
    let mut table = toml::Table::new();
    let mut parts: Vec<&str> = path.split('.').collect();
    let leaf = parts.pop().expect("a leaf");
    let mut cursor = &mut table;
    for part in parts {
        cursor = cursor
            .entry(part)
            .or_insert_with(|| toml::Value::Table(toml::Table::new()))
            .as_table_mut()
            .expect("a table");
    }
    cursor.insert(leaf.to_owned(), value);
    table.to_string()
}

#[test]
fn the_schema_has_the_shape_the_settings_app_loads() {
    let schema = schema();
    assert_eq!(schema["app"].as_str(), Some("actions"));
    assert_eq!(schema["file"].as_str(), Some(SETTINGS_FILE));
    assert_eq!(schema["version"].as_integer(), Some(1));
    let mut seen = BTreeSet::new();
    for row in rows() {
        let p = path(&row);
        assert!(seen.insert(p.to_owned()), "duplicate {p}");
        assert!(p.starts_with("agent."), "{p}");
        for field in ["label", "help", "section"] {
            assert!(!row[field].as_str().expect(field).is_empty(), "{p} {field}");
        }
        assert!(
            matches!(row["exposure"].as_str(), Some("basic" | "advanced")),
            "{p}"
        );
        assert_eq!(row["page"]["kind"].as_str(), Some("intelligence"), "{p}");
        assert!(row.get("agent").is_none(), "{p} is never agent-settable");
        match kind_of(&row) {
            ("bounded", v) => {
                let (min, max) = (
                    v["min"].as_integer().unwrap(),
                    v["max"].as_integer().unwrap(),
                );
                let default = row["default"].as_integer().expect("a number default");
                assert!((min..=max).contains(&default), "{p}");
                assert!(!v["unit"].as_str().unwrap_or("").is_empty(), "{p} unit");
            }
            ("toggle", _) => {
                assert!(row["default"].is_bool(), "{p}");
                assert!(row.get("labels").is_none(), "{p}");
            }
            ("segmented", v) => {
                let words: Vec<&str> = v["variants"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .filter_map(toml::Value::as_str)
                    .collect();
                assert!(words.len() >= 2, "{p}");
                assert!(
                    words.contains(&row["default"].as_str().expect("a word default")),
                    "{p}"
                );
                let labels = row["labels"].as_table().expect("labels for words");
                assert!(words.iter().all(|w| labels.contains_key(*w)), "{p}");
            }
            other => panic!("{p}: kind {other:?}"),
        }
    }
}

#[test]
fn the_intelligence_page_shows_the_assistant_rows_and_the_two_modes_and_the_rest_is_advanced() {
    let shown: Vec<String> = rows()
        .iter()
        .filter(|r| r["exposure"].as_str() == Some("basic"))
        .map(|r| path(r).to_owned())
        .collect();
    assert_eq!(
        shown,
        [
            "agent.start_mode",
            "agent.auto.inside",
            "agent.auto.other",
            "agent.auto.send",
            "agent.auto.delete",
            "agent.auto.screen",
            "agent.manual.inside",
            "agent.manual.other",
            "agent.manual.send",
            "agent.manual.delete",
            "agent.manual.screen",
            "agent.mcp.expose",
            "agent.acp.agents"
        ]
    );
}

#[test]
fn the_schema_rows_and_the_key_table_are_the_same_keys_with_the_same_ranges() {
    let keys = table();
    let rows = rows();
    let schema_paths: Vec<&str> = rows.iter().map(path).collect();
    let code_paths: Vec<&str> = keys.iter().map(|k| k.path).collect();
    assert_eq!(schema_paths, code_paths);
    for (row, key) in rows.iter().zip(&keys) {
        let (_, v) = kind_of(row);
        match &key.rule {
            Rule::Number { range, .. } => {
                assert_eq!(v["min"].as_integer(), Some(*range.start()), "{}", key.path);
                assert_eq!(v["max"].as_integer(), Some(*range.end()), "{}", key.path);
            }
            Rule::Toggle { .. } => assert_eq!(kind_of(row).0, "toggle", "{}", key.path),
            Rule::Word { words, set } => {
                for word in *words {
                    assert!(
                        set(&mut AgentSettings::default(), word),
                        "{} {word}",
                        key.path
                    );
                }
                let schema_words: Vec<&str> = v["variants"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .filter_map(toml::Value::as_str)
                    .collect();
                assert_eq!(&schema_words, words, "{}", key.path);
            }
        }
    }
}

#[test]
fn a_file_of_every_schema_default_changes_nothing() {
    let mut text = toml::Table::new();
    for row in rows() {
        let one: toml::Table = doc(path(&row), row["default"].clone()).parse().unwrap();
        for (k, v) in one {
            merge(&mut text, k, v);
        }
    }
    let loaded = read(&text.to_string(), AgentSettings::default());
    assert_eq!(loaded.value, AgentSettings::default());
    assert_eq!((loaded.invalid, loaded.unknown), (vec![], vec![]));
}

fn merged(parts: &[(&str, toml::Value)]) -> String {
    let mut text = toml::Table::new();
    for (p, v) in parts {
        for (k, v) in doc(p, v.clone()).parse::<toml::Table>().unwrap() {
            merge(&mut text, k, v);
        }
    }
    text.to_string()
}

fn merge(into: &mut toml::Table, key: String, value: toml::Value) {
    match (into.get_mut(&key), value) {
        (Some(toml::Value::Table(have)), toml::Value::Table(more)) => {
            for (k, v) in more {
                merge(have, k, v);
            }
        }
        (_, value) => {
            into.insert(key, value);
        }
    }
}

/// A valid value for the row that is not its default.
fn other_than_default(row: &toml::Table) -> toml::Value {
    match kind_of(row) {
        ("toggle", _) => toml::Value::Boolean(!row["default"].as_bool().unwrap()),
        (_, v) if v.contains_key("min") => {
            let default = row["default"].as_integer().unwrap();
            let (min, max) = (
                v["min"].as_integer().unwrap(),
                v["max"].as_integer().unwrap(),
            );
            toml::Value::Integer(if default == max { min } else { max })
        }
        (_, v) => {
            let default = row["default"].as_str().unwrap();
            let word = v["variants"]
                .as_array()
                .unwrap()
                .iter()
                .filter_map(toml::Value::as_str)
                .find(|w| *w != default)
                .unwrap();
            toml::Value::String(word.to_owned())
        }
    }
}

#[test]
fn every_schema_key_is_read_by_the_daemon() {
    for row in rows() {
        let p = path(&row);
        let loaded = read(&doc(p, other_than_default(&row)), AgentSettings::default());
        assert_eq!(loaded.invalid, vec![], "{p}");
        assert_ne!(
            loaded.value,
            AgentSettings::default(),
            "{p} changed nothing"
        );
    }
}

#[test]
fn each_key_lands_in_the_typed_value_the_design_names() {
    use permission_core::{AskOrRun, Mode, OtherRow, ScreenRow};
    let read_one = |p: &str, v: toml::Value| read(&doc(p, v), AgentSettings::default()).value;
    let int = toml::Value::Integer;
    let word = |w: &str| toml::Value::String(w.to_owned());
    assert_eq!(
        read_one("agent.start_mode", word("manual"))
            .agent
            .start_mode,
        Mode::Manual
    );
    // Each row of each mode lands in its own place.
    let rows_of = |mode: &str, row: &str, value: &str| {
        read_one(&format!("agent.{mode}.{row}"), word(value))
            .agent
            .rows
    };
    assert_eq!(
        rows_of("auto", "inside", "asks").auto.inside,
        AskOrRun::Asks
    );
    assert_eq!(rows_of("auto", "other", "asks").auto.other, OtherRow::Asks);
    assert_eq!(rows_of("auto", "send", "runs").auto.send, AskOrRun::Runs);
    assert_eq!(
        rows_of("auto", "delete", "runs").auto.delete,
        AskOrRun::Runs
    );
    assert_eq!(
        rows_of("auto", "screen", "each").auto.screen,
        ScreenRow::Each
    );
    assert_eq!(
        rows_of("manual", "inside", "runs").manual.inside,
        AskOrRun::Runs
    );
    assert_eq!(
        rows_of("manual", "other", "check").manual.other,
        OtherRow::Check
    );
    assert_eq!(
        rows_of("manual", "send", "runs").manual.send,
        AskOrRun::Runs
    );
    assert_eq!(
        rows_of("manual", "delete", "runs").manual.delete,
        AskOrRun::Runs
    );
    assert_eq!(
        rows_of("manual", "screen", "once").manual.screen,
        ScreenRow::Once
    );
    let on = toml::Value::Boolean(true);
    assert!(read_one("agent.mcp.expose", on.clone()).expose);
    assert!(read_one("agent.acp.expose", on.clone()).acp);
    assert!(read_one("agent.acp.agents", on).agents);
    assert_eq!(
        read_one("agent.budget.spend_microusd", int(2_500_000))
            .agent
            .budget
            .spend,
        Some(micro_usd(2_500_000))
    );
}

#[test]
fn a_bad_value_falls_back_for_that_key_alone() {
    for row in rows() {
        let p = path(&row);
        let bad: Vec<toml::Value> = match kind_of(&row) {
            ("toggle", _) => vec![toml::Value::String("on".into()), toml::Value::Integer(1)],
            (_, v) if v.contains_key("min") => vec![
                toml::Value::Integer(v["min"].as_integer().unwrap() - 1),
                toml::Value::Integer(v["max"].as_integer().unwrap() + 1),
                toml::Value::String("many".into()),
                toml::Value::Boolean(true),
            ],
            _ => vec![
                toml::Value::String("reckless".into()),
                toml::Value::Integer(1),
                toml::Value::Boolean(true),
            ],
        };
        for value in bad {
            // A second, good key in the same file still applies.
            let spend = "agent.budget.spend_microusd";
            let (other, good) = if p == spend {
                ("agent.start_mode", toml::Value::String("manual".into()))
            } else {
                (spend, toml::Value::Integer(100))
            };
            let text = merged(&[(p, value.clone()), (other, good)]);
            let loaded = read(&text, AgentSettings::default());
            let mut want = AgentSettings::default();
            if other == spend {
                want.agent.budget.spend = Some(micro_usd(100));
            } else {
                want.agent.start_mode = permission_core::Mode::Manual;
            }
            assert_eq!(loaded.value, want, "{p} = {value}");
            assert_eq!(loaded.invalid.len(), 1, "{p} = {value}");
            assert_eq!(loaded.invalid[0].path, p);
        }
    }
}

#[test]
fn a_fallback_is_the_base_value_not_the_default() {
    let mut base = AgentSettings::default();
    base.agent.budget.spend = Some(micro_usd(11));
    let loaded = read("[agent.budget]\nspend_microusd = -1\n", base);
    assert_eq!(loaded.value, base);
    assert_eq!(
        loaded.invalid,
        vec![InvalidKey {
            path: "agent.budget.spend_microusd".into(),
            reason: "outside 0..=1000000000".into(),
        }]
    );
}

#[test]
fn a_file_that_is_not_toml_keeps_every_base_value_and_says_so() {
    let loaded = read("[agent.mcp\nexpose = true", AgentSettings::default());
    assert_eq!(loaded.value, AgentSettings::default());
    assert_eq!(loaded.invalid.len(), 1);
    assert!(loaded.invalid[0].reason.starts_with("not TOML"));
}

#[test]
fn unknown_keys_are_reported_and_version_is_not_one() {
    let loaded = read(
        "version = 1\n[agent.budget]\nbogus = 1\n[agent.mcp]\nexpose = true\n[other]\nx = 1\n",
        AgentSettings::default(),
    );
    assert!(loaded.value.expose);
    assert_eq!(
        loaded.unknown,
        [
            UnknownKey("agent.budget.bogus".to_owned()),
            UnknownKey("other.x".to_owned())
        ]
    );
    assert_eq!(
        loaded.lines("actiond"),
        [
            "actiond: settings: agent.budget.bogus: no such key, ignored",
            "actiond: settings: other.x: no such key, ignored"
        ]
    );
}
