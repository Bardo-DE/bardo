//! Telling a worker to give up: a flag the caller raises and an optional deadline.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Instant;

/// A request to end work early, handed to `Backend::run`.
///
/// Clones share one flag, so the caller keeps a clone and raises it while a worker polls its own.
/// The deadline is plain data and the type's own methods never read a clock: the question "is it
/// past?" takes the instant (`stopped_at`), and the worker passes `Instant::now()`. A loop that
/// does not hold a clock asks it as an [`Interrupt`](super::Interrupt), whose impl for `Stop` reads
/// one. A back end with its own deadline (a per-render limit) reads `deadline` and uses that
/// instead.
#[derive(Debug, Clone, Default)]
pub struct Stop {
    flag: Arc<AtomicBool>,
    deadline: Option<Instant>,
}

impl Stop {
    /// A stop that ends only when `request` is called.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// A stop that also ends at `deadline`.
    #[must_use]
    pub fn with_deadline(deadline: Instant) -> Self {
        Self {
            flag: Arc::default(),
            deadline: Some(deadline),
        }
    }

    /// Asks every clone's work to end.
    pub fn request(&self) {
        self.flag.store(true, Ordering::Release);
    }

    /// Whether `request` was called on this or any clone; ignores the deadline.
    #[must_use]
    pub fn stopped(&self) -> bool {
        self.flag.load(Ordering::Acquire)
    }

    /// Whether the work should end at `now`: asked to, or `now` is at or past the deadline.
    #[must_use]
    pub fn stopped_at(&self, now: Instant) -> bool {
        self.stopped() || self.deadline.is_some_and(|deadline| now >= deadline)
    }

    /// The flag itself, shared with every clone: for back ends whose own stop polls a flag.
    /// Raising it is `request`; it is never lowered.
    #[must_use]
    pub fn flag(&self) -> Arc<AtomicBool> {
        Arc::clone(&self.flag)
    }

    /// When the work must end, if it has a deadline.
    #[must_use]
    pub fn deadline(&self) -> Option<Instant> {
        self.deadline
    }
}
