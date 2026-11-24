//! Waits with a deadline.
//!
//! Every wait of a test has a deadline ([`within`], [`DEADLINE`]): a wait that does not end is a
//! failure that names what it waited for, not a hang until the harness ends the test.

use std::time::Duration;

/// How long any one wait of a test may take. A wait ends in milliseconds, in seconds on a loaded
/// machine, in tens of seconds on one with its swap full; the harness ends a whole test after
/// ten minutes.
pub const DEADLINE: Duration = base_testkit::GENEROUS;

/// What `future` gives, or a panic naming `what` once [`DEADLINE`] has passed.
pub async fn within<T>(what: &str, future: impl Future<Output = T>) -> T {
    match tokio::time::timeout(DEADLINE, future).await {
        Ok(out) => out,
        Err(elapsed) => panic!("waited {} s for {what} ({elapsed})", DEADLINE.as_secs()),
    }
}
