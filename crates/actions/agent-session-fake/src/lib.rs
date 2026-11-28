//! Test support for `agent-session`, kept out of its production API.
//!
//! - [`contract`]: the rules every `SessionBackend` keeps, written once for each backend's test
//!   to run. Its checks panic by design.
//! - [`fake`]: a scripted backend and an in-memory log.
//! - [`fake_host`]: a scripted host over those two.

pub mod contract;
pub mod fake;
pub mod fake_host;
