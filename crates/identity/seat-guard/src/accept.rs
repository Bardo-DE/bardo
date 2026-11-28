//! What a failed `accept` means.

use rustix::io::Errno;
use seat_guard_core::AcceptFailure;

/// What the failed `accept` that reported `e` means.
#[must_use]
pub fn accept_failure(e: Errno) -> AcceptFailure {
    match e {
        Errno::AGAIN => AcceptFailure::NothingPending,
        Errno::INTR => AcceptFailure::Retry,
        _ => AcceptFailure::Pause,
    }
}
