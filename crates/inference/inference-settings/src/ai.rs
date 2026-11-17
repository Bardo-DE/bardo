//! The `[ai]` table of `inferd.toml`: every settings row of the `ai` domain at its settings path.
//! The structured-output rows inside it are [`StructuredConfig`]; the rest are read by `resolve`.

use crate::structured::{Resolved, StructuredConfig};
use serde::{Deserialize, Serialize};

/// The `[ai]` table: every settings row of the `ai` domain at its settings path.
///
/// The `[ai]` table: every settings row of the `ai` domain at its settings path, as a settings
/// writer lays a file out (`ai.local_only`, `ai.floor.<class>`, `ai.model.<kind>.<tier>`,
/// `ai.auto.*`, `ai.spend.*`, `ai.length.*`, `ai.structured.*`). Values are the rows' slugs, resolved (and
/// refused field by field) by `resolve`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct AiConfig {
    /// `ai.local_only`: while on, every cloud account is removed from every answer; off, cloud
    /// accounts may serve classes whose floor allows it.
    #[serde(default)]
    pub local_only: Option<crate::SwitchSaid>,
    /// `ai.floor.<class>`: `on_device`, `local_network` or `anywhere`, by class slug.
    #[serde(default)]
    pub floor: std::collections::BTreeMap<String, String>,
    /// `ai.model.<slot>.<tier>`: `""`, `auto` or `<account>/<model>`, by slot slug, then tier.
    #[serde(default)]
    pub model: std::collections::BTreeMap<String, std::collections::BTreeMap<String, String>>,
    /// `ai.spend.*`.
    #[serde(default)]
    pub spend: crate::SpendConfig,
    /// `ai.length.*`.
    #[serde(default)]
    pub length: crate::LengthConfig,
    /// `ai.pipeline.*`.
    #[serde(default)]
    pub pipeline: crate::PipelineConfig,
    /// `ai.structured.*`.
    #[serde(default)]
    pub structured: StructuredConfig,
    /// `ai.auto.*`.
    #[serde(default)]
    pub auto: crate::AutoConfig,
    /// `ai.attached.*`.
    #[serde(default)]
    pub attached: crate::AttachedConfig,
    /// `ai.agents.*`.
    #[serde(default)]
    pub agents: crate::AgentsConfig,
    /// `ai.tailnet.*`.
    #[serde(default)]
    pub tailnet: crate::TailnetConfig,
}

impl AiConfig {
    /// The structured-output limits for this configuration.
    #[must_use]
    pub fn resolve(&self) -> Resolved {
        self.structured.resolve()
    }
}
