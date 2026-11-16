//! A private session bus for one test: one shared helper, whose daemon is killed by PID when the
//! bus drops (see `action-testbus`). The person's real bus is never named.

#![allow(
    unused_imports,
    reason = "each test binary uses a different part of the shared support"
)]

pub use action_testbus::PrivateBus;
