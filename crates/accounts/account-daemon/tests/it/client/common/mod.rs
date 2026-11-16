//! Shared by the bus tests: a private bus and the real accountd on it.
#![allow(dead_code, reason = "helpers shared by the test files; each file uses some of them")]

pub mod accountd;
pub mod bus;
pub mod fake_accountd;

/// Waits until `condition` holds (a daemon's task has run); fails the test, naming `what`, when
/// `base_testkit::GENEROUS` has passed by the clock.
pub async fn eventually(what: &str, mut condition: impl FnMut() -> bool) {
    let deadline = base_testkit::Deadline::generous();
    while !deadline.passed() {
        if condition() {
            return;
        }
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
    }
    deadline.fail(what);
}
