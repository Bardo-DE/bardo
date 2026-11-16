//! The clock: when a file was last viewed is handed in by the caller, in seconds since the Unix
//! epoch (the store itself reads no clock; [`system_clock`] is the one a program hands in).

/// The time as a program reads it, in seconds since the Unix epoch, the unit the launcher's own
/// clock counts in: the one place a program asks the time, so a test passes a fixed one.
pub type Clock = std::sync::Arc<dyn Fn() -> u64 + Send + Sync>;

/// The wall clock in whole seconds since the epoch; a clock set before it reads as the epoch itself.
#[must_use]
pub fn system_clock() -> Clock {
    use base_time::Clock as _;
    std::sync::Arc::new(|| u64::try_from(base_time::SystemClock.now().0).unwrap_or(0))
}
