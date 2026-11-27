//! The wall clock and the per-computer jitter seed; the only readers of either.

use base_clock::SystemClock;
use base_time::{Clock, UnixSeconds};
use updater_core::Sha256Hex;

/// The time now.
#[must_use]
pub fn now() -> UnixSeconds {
    SystemClock.now()
}

/// A stable number for this computer from its machine id text (any text will do).
#[must_use]
pub fn jitter_seed(machine_id: &str) -> u64 {
    let digest = Sha256Hex::of(machine_id.trim().as_bytes());
    u64::from_str_radix(&digest.as_str()[..16], 16).unwrap_or(0)
}
