//! Bus names, object path and interface names.

/// memoryd's bus name (session bus, activatable).
pub const MEMORY_BUS: &str = "org.bardo.Memory1";
/// memoryd's one object.
pub const MEMORY_PATH: &str = "/org/bardo/Memory1";
/// Writers: record, explain, mark.
pub const RECORD_INTERFACE: &str = "org.bardo.Memory1.Record";
/// Readers: search, facts, related, provenance, primer, propose.
pub const RECALL_INTERFACE: &str = "org.bardo.Memory1.Recall";
/// The person's controls: timeline, forget, pending, consolidation, rules, export.
pub const CONTROL_INTERFACE: &str = "org.bardo.Memory1.Control";
/// The prefix of every error name; the variant follows (`...Error.PlanStale`).
pub const ERROR_PREFIX: &str = "org.bardo.Memory1.Error";
