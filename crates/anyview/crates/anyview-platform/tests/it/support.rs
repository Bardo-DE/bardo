//! What the tests of this crate share.

// Each test file uses some of these helpers, so an unused one is not dead code.
#![allow(
    dead_code,
    reason = "helpers shared by the test files; each file uses some of them"
)]

use anyview_platform::{BusRoute, Env};
use base_testkit::PrivateBus;

/// An environment whose session bus is `bus` and whose directories are scratch.
pub fn env_of(bus: &PrivateBus) -> Env {
    Env::isolated(bus.scratch()).with_session(BusRoute::Address(bus.address().to_owned()))
}
