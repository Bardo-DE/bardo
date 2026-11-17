//! The lenient reader (design/22 section 2): a bad value falls back to the base value for that
//! key alone, never fails the file, and is reported so the daemon can log it. The scalar keys are
//! read by the shared table-driven reader of `settings-store`; the list `cua.apps` is read here,
//! row by row.

use super::CuaSettings;
use super::apps::{AppAllow, AppRule, Apps, is_browser};
use super::keys::{APPS_KEY, table};
use account_core::AppName;
use settings_store::rules::{RuleTable, find, read_rules};
use settings_store::{InvalidKey, Loaded};

/// The reason of a row with no usable `app`.
const NOT_AN_APP: &str = "no desktop-entry id in `app`";

/// One row of `cua.apps`: the rule, and what was changed or refused about it.
fn row(at: usize, value: &toml::Value) -> (Option<AppRule>, Option<InvalidKey>) {
    let refuse = |reason: &str| {
        (
            None,
            Some(InvalidKey {
                path: format!("{APPS_KEY}[{at}]"),
                reason: reason.to_owned(),
            }),
        )
    };
    let Some(table) = value.as_table() else {
        return refuse(NOT_AN_APP);
    };
    let Some(app) = table
        .get("app")
        .and_then(toml::Value::as_str)
        .and_then(|id| AppName::parse(id).ok())
    else {
        return refuse(NOT_AN_APP);
    };
    let Some(allow) = table
        .get("allow")
        .and_then(toml::Value::as_str)
        .and_then(AppAllow::of_word)
    else {
        return refuse("`allow` is not ask, session or always");
    };
    if allow == AppAllow::Always && is_browser(&app) {
        let rule = AppRule {
            app,
            allow: AppAllow::Ask,
        };
        let why = InvalidKey {
            path: format!("{APPS_KEY}[{at}]"),
            reason: "a browser is never `always`; it asks every run".to_owned(),
        };
        return (Some(rule), Some(why));
    }
    (Some(AppRule { app, allow }), None)
}

/// `cua.apps` as the file has it: a list of rows, each read on its own.
fn apps(value: &toml::Value) -> (Option<Apps>, Vec<InvalidKey>) {
    let Some(rows) = value.as_array() else {
        let why = InvalidKey {
            path: APPS_KEY.to_owned(),
            reason: "not a list of { app, allow } tables".to_owned(),
        };
        return (None, vec![why]);
    };
    let mut rules: Vec<AppRule> = Vec::new();
    let mut invalid = Vec::new();
    for (at, value) in rows.iter().enumerate() {
        let (rule, refused) = row(at, value);
        invalid.extend(refused);
        // A later row for the same app replaces the earlier one.
        if let Some(rule) = rule {
            rules.retain(|r| r.app != rule.app);
            rules.push(rule);
        }
    }
    (Some(Apps::Listed(rules)), invalid)
}

/// Reads `text` over `base`: each key the file sets and the table accepts replaces its base value;
/// every other key keeps it.
///
/// The daemon's `base` lists no app, so until the file names some, none is on (design/22 section
/// 3.29).
#[must_use]
pub fn read(text: &str, base: CuaSettings) -> Loaded<CuaSettings> {
    let keys = table();
    let rules = RuleTable {
        keys: &keys,
        also_known: &[APPS_KEY],
        ignored_prefixes: &[],
    };
    read_rules(text, base, &rules, |doc, value, invalid| {
        if let Some(found) = find(doc, APPS_KEY) {
            let (listed, refused) = apps(found);
            value.apps = listed.unwrap_or_else(|| value.apps.clone());
            invalid.extend(refused);
        }
    })
}
