//! The one poll for a condition that has no event to wait on. Prefer waiting on the event
//! itself (a channel, a signal, a property change) or a paused clock; poll only where the thing
//! watched is outside the test's reach (another process's state, a file another program writes).

use crate::Deadline;
use std::time::Duration;

/// How often a poll looks again: often enough not to slow a test, rare enough not to load it.
const LOOK_AGAIN: Duration = Duration::from_millis(10);

/// Waits until `condition` holds, looking again every few milliseconds, up to
/// [`crate::GENEROUS`] by the clock.
///
/// # Panics
/// When the condition does not hold in time, naming `what`.
#[cfg(feature = "tokio")]
pub async fn eventually(what: &str, mut condition: impl FnMut() -> bool) {
    let deadline = Deadline::generous();
    while !condition() {
        if deadline.passed() {
            deadline.fail(what);
        }
        tokio::time::sleep(LOOK_AGAIN).await;
    }
}

/// Waits until `look` gives a value, looking again every few milliseconds, up to
/// [`crate::GENEROUS`] by the clock, and returns it.
///
/// # Panics
/// When no value comes in time, naming `what`.
#[cfg(feature = "tokio")]
pub async fn eventually_some<T>(what: &str, mut look: impl FnMut() -> Option<T>) -> T {
    let deadline = Deadline::generous();
    loop {
        if let Some(value) = look() {
            return value;
        }
        if deadline.passed() {
            deadline.fail(what);
        }
        tokio::time::sleep(LOOK_AGAIN).await;
    }
}

/// The blocking form of [`eventually`], for tests without a runtime.
///
/// # Panics
/// When the condition does not hold in time, naming `what`.
pub fn eventually_blocking(what: &str, mut condition: impl FnMut() -> bool) {
    let deadline = Deadline::generous();
    while !condition() {
        if deadline.passed() {
            deadline.fail(what);
        }
        std::thread::sleep(LOOK_AGAIN);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;

    #[test]
    fn a_condition_that_comes_true_ends_the_wait() {
        let looks = Cell::new(0);
        eventually_blocking("the third look", || {
            looks.set(looks.get() + 1);
            looks.get() == 3
        });
        assert_eq!(looks.get(), 3);
    }
}
