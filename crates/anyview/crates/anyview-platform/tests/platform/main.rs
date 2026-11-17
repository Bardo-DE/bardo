//! The integration tests of anyview-platform, in one binary so they link once.

mod agent_intents;
mod single_instance;
mod socket_instance;
#[cfg(feature = "desktop")]
mod support;
