//! Locking that survives a panic elsewhere.

use std::sync::{Mutex, MutexGuard, PoisonError};

/// The guard of `mutex`. A task that panicked while holding it has poisoned it, and every later
/// call would panic too; each update here leaves its data whole, so the data is taken as it is.
pub(crate) fn locked<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}
