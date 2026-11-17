//! The integration tests of anyview-platform, in one binary so they link once.

mod agent_intents;
mod latchkey_instance;
mod single_instance;
#[cfg(feature = "quire-desktop")]
mod support;
