//! The key table: every `memory.*` key of the schema (`schema/memory.settings.toml`), its
//! rule and where its value goes. The tests hold the schema and this table to each other.

use crate::{ConsolidateApply, ConsolidateWhen, MemorySettings};
use memory_core::{DayCount, VaultKind};
use settings_store::rules::{RuleKey, number, word};

fn days(v: i64) -> DayCount {
    DayCount(u32::try_from(v).unwrap_or(u32::MAX))
}

const AT_REST: [VaultKind; 2] = [VaultKind::Sealed, VaultKind::Plain];
const WHEN: [ConsolidateWhen; 3] = [
    ConsolidateWhen::Nightly,
    ConsolidateWhen::Manual,
    ConsolidateWhen::Never,
];

const APPLY: [ConsolidateApply; 2] = [ConsolidateApply::Auto, ConsolidateApply::Review];

const AT_REST_WORDS: &[&str] = &["sealed", "plain"];
const WHEN_WORDS: &[&str] = &["nightly", "manual", "never"];
const APPLY_WORDS: &[&str] = &["auto", "review"];

/// The value of `values` whose word in `words` is `given`.
fn pick<V: Copy>(words: &[&str], values: &[V], given: &str) -> Option<V> {
    words
        .iter()
        .position(|word| *word == given)
        .map(|at| values[at])
}

/// Every key, in the order the schema lists them.
pub(crate) fn table() -> Vec<RuleKey<MemorySettings>> {
    vec![
        word("memory.files.at_rest", AT_REST_WORDS, |s, w| {
            pick(AT_REST_WORDS, &AT_REST, w).is_some_and(|v| {
                s.at_rest = v;
                true
            })
        }),
        word("memory.consolidation.when", WHEN_WORDS, |s, w| {
            pick(WHEN_WORDS, &WHEN, w).is_some_and(|v| {
                s.consolidate = v;
                true
            })
        }),
        word("memory.consolidation.apply", APPLY_WORDS, |s, w| {
            pick(APPLY_WORDS, &APPLY, w).is_some_and(|v| {
                s.apply = v;
                true
            })
        }),
        number("memory.retention.search_days", 1..=3650, |s, v| {
            s.retention.search = days(v);
        }),
        number("memory.retention.session_days", 1..=3650, |s, v| {
            s.retention.session = days(v);
        }),
    ]
}
