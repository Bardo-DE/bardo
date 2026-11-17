//! The lenient reader (design/22 section 2): a bad value falls back to the base value for that
//! key alone, never fails the file, and is reported so the daemon can log it. The reading is the
//! shared table-driven reader of `settings-store`; this is the key table of `agent.*`.

use crate::AgentSettings;
use crate::keys::table;
use settings_store::Loaded;
use settings_store::rules::{RuleTable, read_rules};

/// Reads `text` over `base`: each key the file sets and the table accepts replaces its base
/// value; every other key keeps it.
#[must_use]
pub fn read(text: &str, base: AgentSettings) -> Loaded<AgentSettings> {
    let keys = table();
    let rules = RuleTable {
        keys: &keys,
        also_known: &[],
        ignored_prefixes: &["assistant."],
    };
    read_rules(text, base, &rules, |_, _, _| {})
}
