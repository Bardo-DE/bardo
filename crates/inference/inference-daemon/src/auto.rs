//! The `ai.auto.*` settings rows (design/22 section 3.26): what Automatic does, whether it may
//! unload an idle model, whether it says why.
//!
//! The daemon reads `[ai.auto]` of `inferd.toml`; `settings::resolve` calls [`AutoConfig::resolve`]
//! and puts the policy into the settings the engines run under (`Engines::with_settings`). Nothing
//! here reads a file or the environment. A value a row does not know falls back to the row's
//! default, for that field only, and is named in [`ResolvedAuto::rejected`] so the daemon can log
//! it once (as `[ai.structured]` does).

use crate::settings::SwitchSaid;
use inference_core::{AutoEvict, AutoMode, AutoPolicy};
use serde::{Deserialize, Serialize};

/// `ai.auto.mode`.
pub const MODE: &str = "ai.auto.mode";
/// `ai.auto.allow_evict`.
pub const ALLOW_EVICT: &str = "ai.auto.allow_evict";
/// `ai.auto.show_reason`.
pub const SHOW_REASON: &str = "ai.auto.show_reason";

/// The `[ai.auto]` table, as written: each value is the row's slug.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct AutoConfig {
    /// `ai.auto.mode`: `warm_first`.
    #[serde(default)]
    pub mode: Option<String>,
    /// `ai.auto.allow_evict`: `never` or `idle_only`.
    #[serde(default)]
    pub allow_evict: Option<String>,
    /// `ai.auto.show_reason`: whether every answer says why (on by default).
    #[serde(default)]
    pub show_reason: Option<SwitchSaid>,
    /// `ai.auto.pool.<slot>`: the `<account>/<model>`s Automatic chooses among, by slot slug.
    #[serde(default)]
    pub pool: std::collections::BTreeMap<String, Vec<String>>,
}

/// The rows in force, and the paths of the ones whose values were refused.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedAuto {
    /// What Automatic runs under.
    pub policy: AutoPolicy,
    /// The rows that held an unknown value and fell back to their default.
    pub rejected: Vec<&'static str>,
}

fn pick<T: Default>(
    said: Option<&str>,
    parse: impl Fn(&str) -> Option<T>,
    path: &'static str,
) -> (T, Option<&'static str>) {
    match said {
        None => (T::default(), None),
        Some(text) => match parse(text) {
            Some(value) => (value, None),
            None => (T::default(), Some(path)),
        },
    }
}

impl AutoConfig {
    /// The rows for this configuration.
    pub fn resolve(&self) -> ResolvedAuto {
        let (mode, a) = pick(self.mode.as_deref(), AutoMode::from_slug, MODE);
        let (allow_evict, b) = pick(
            self.allow_evict.as_deref(),
            AutoEvict::from_slug,
            ALLOW_EVICT,
        );
        let (show_reason, c) = match &self.show_reason {
            None => (true, None),
            Some(SwitchSaid::Switch(on)) => (*on, None),
            Some(SwitchSaid::Word(_)) => (true, Some(SHOW_REASON)),
        };
        ResolvedAuto {
            policy: AutoPolicy {
                mode,
                allow_evict,
                show_reason,
            },
            rejected: [a, b, c].into_iter().flatten().collect(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::InferdConfig;

    #[test]
    fn no_table_is_the_design_defaults() {
        let resolved = InferdConfig::default().ai.auto.resolve();
        assert_eq!(resolved.policy, AutoPolicy::default());
        assert_eq!(resolved.policy.allow_evict, AutoEvict::IdleOnly);
        assert!(resolved.policy.show_reason);
        assert!(resolved.rejected.is_empty());
    }

    #[test]
    fn the_table_reads_from_the_file() {
        let config = InferdConfig::from_toml(
            "[ai.auto]\nmode = \"warm_first\"\nallow_evict = \"never\"\nshow_reason = false\n",
        )
        .expect("reads");
        let resolved = config.ai.auto.resolve();
        assert_eq!(resolved.policy.allow_evict, AutoEvict::Never);
        assert!(!resolved.policy.show_reason);
        assert!(resolved.rejected.is_empty());
    }

    #[test]
    fn an_unknown_value_falls_back_for_that_field_only() {
        let config = InferdConfig::from_toml(
            "[ai.auto]\nallow_evict = \"sometimes\"\nshow_reason = \"maybe\"\n",
        )
        .expect("reads");
        let resolved = config.ai.auto.resolve();
        assert_eq!(resolved.policy.allow_evict, AutoEvict::IdleOnly);
        assert!(resolved.policy.show_reason);
        assert_eq!(resolved.rejected, vec![ALLOW_EVICT, SHOW_REASON]);
    }
}
