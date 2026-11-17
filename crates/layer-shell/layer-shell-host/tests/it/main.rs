//! All integration tests, one binary. No test here sets process-global state, so none needs its
//! own process. `gpu/tests.rs` shares one device through a `OnceLock`, so it stays a unit test of
//! the lib.

#[cfg(feature = "headless")]
mod headless;

// The examples' unit tests. They live beside the code they test; the examples themselves do not
// need to link the crate a second time to run them.
#[cfg(feature = "debug-inject")]
#[path = "../../examples/shared/dock.rs"]
#[allow(
    dead_code,
    reason = "the module is shared with the demos; the tests use part of it"
)]
mod dock;
#[allow(
    dead_code,
    reason = "the module is shared with the demo; the tests use part of it"
)]
#[path = "../../examples/demo_lock/guard.rs"]
mod guard;
