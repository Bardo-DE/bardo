//! The key table: every scalar `cua.*` key of design/22 section 3.29 that cuad reads, its rule and
//! where its value goes. One row per schema row (`dist/settings/computer-use.settings.toml`) except
//! `cua.apps`, which is a list and is read in `apps.rs`/`read.rs`; the tests hold the schema and
//! this table to each other.

use super::CuaSettings;
use model_provider::Money;
use rust_decimal::Decimal;
use serde::de::DeserializeOwned;
use settings_store::rules::{RuleKey, number, toggle, word};

/// The list key, read apart from the table.
pub(crate) const APPS_KEY: &str = "cua.apps";

/// Reads `word` as the serde form of `T`; `None` when it is not one of `T`'s words.
fn read_word<T: DeserializeOwned>(word: &str) -> Option<T> {
    toml::Value::String(word.to_owned()).try_into().ok()
}

/// Every scalar key, in the order the schema lists them.
pub(crate) fn table() -> Vec<RuleKey<CuaSettings>> {
    vec![
        toggle("cua.enabled", |s, v| s.config.enabled = v),
        word(
            "cua.default_mode",
            &["auto", "in_place", "agent_workspace"],
            |s, w| read_word(w).map(|v| s.config.default_mode = v).is_some(),
        ),
        toggle("cua.autopause", |s, v| s.config.autopause = v),
        toggle("cua.accessibility", |s, v| s.config.accessibility = v),
        number("cua.budget.spend_microusd", 0..=1_000_000_000, |s, v| {
            // Millionths of a US dollar in the file; 0 is a cap of nothing (local models only).
            s.config.budget.spend = Money::usd(Decimal::new(v, 6));
        }),
    ]
}
