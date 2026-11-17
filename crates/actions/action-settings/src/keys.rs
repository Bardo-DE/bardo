//! The key table: every `agent.*` key of design/22 section 3.27, its rule and where its value
//! goes. One row per schema row (`dist/settings/actions.settings.toml`); the tests hold the two
//! tables to each other, so a row without a reader or a reader without a row fails.

use crate::AgentSettings;
use model_provider::Money;
use rust_decimal::Decimal;
use serde::Deserialize;
use serde::de::value::{Error as DeError, StrDeserializer};
use serde::de::{DeserializeOwned, IntoDeserializer};
use settings_store::rules::{RuleKey, number, toggle, word};

/// Reads `word` into `slot` by the type's serde name; whether it was one.
fn parse_into<T: DeserializeOwned>(slot: &mut T, word: &str) -> bool {
    let de: StrDeserializer<'_, DeError> = word.into_deserializer();
    match <T as Deserialize<'_>>::deserialize(de) {
        Ok(value) => {
            *slot = value;
            true
        }
        Err(_) => false,
    }
}

/// The start mode and the ten rows of the two modes.
fn mode_keys() -> Vec<RuleKey<AgentSettings>> {
    let inside: &[&str] = &["runs", "asks"];
    let other: &[&str] = &["check", "asks"];
    let ask_or_runs: &[&str] = &["asks", "runs"];
    let screen: &[&str] = &["once", "each"];
    vec![
        word("agent.start_mode", &["auto", "manual"], |s, w| {
            parse_into(&mut s.agent.start_mode, w)
        }),
        word("agent.auto.inside", inside, |s, w| {
            parse_into(&mut s.agent.rows.auto.inside, w)
        }),
        word("agent.auto.other", other, |s, w| {
            parse_into(&mut s.agent.rows.auto.other, w)
        }),
        word("agent.auto.send", ask_or_runs, |s, w| {
            parse_into(&mut s.agent.rows.auto.send, w)
        }),
        word("agent.auto.delete", ask_or_runs, |s, w| {
            parse_into(&mut s.agent.rows.auto.delete, w)
        }),
        word("agent.auto.screen", screen, |s, w| {
            parse_into(&mut s.agent.rows.auto.screen, w)
        }),
        word("agent.manual.inside", inside, |s, w| {
            parse_into(&mut s.agent.rows.manual.inside, w)
        }),
        word("agent.manual.other", other, |s, w| {
            parse_into(&mut s.agent.rows.manual.other, w)
        }),
        word("agent.manual.send", ask_or_runs, |s, w| {
            parse_into(&mut s.agent.rows.manual.send, w)
        }),
        word("agent.manual.delete", ask_or_runs, |s, w| {
            parse_into(&mut s.agent.rows.manual.delete, w)
        }),
        word("agent.manual.screen", screen, |s, w| {
            parse_into(&mut s.agent.rows.manual.screen, w)
        }),
    ]
}

/// Every key, in the order the schema lists them.
pub(crate) fn table() -> Vec<RuleKey<AgentSettings>> {
    let mut keys = mode_keys();
    keys.extend(rest());
    keys
}

fn rest() -> Vec<RuleKey<AgentSettings>> {
    vec![
        toggle("agent.mcp.expose", |s, on| s.expose = on),
        toggle("agent.acp.expose", |s, on| s.acp = on),
        toggle("agent.acp.agents", |s, on| s.agents = on),
        number("agent.budget.spend_microusd", 0..=1_000_000_000, |s, v| {
            // Millionths of a US dollar in the file. 0 is a cap of nothing: the turn after any
            // reported charge stops (a free or unpriced turn reports none).
            s.agent.budget.spend = Some(Money::usd(Decimal::new(v, 6)));
        }),
    ]
}
