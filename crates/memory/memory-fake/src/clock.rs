//! The instant every fixture uses.

use memory_core::UnixSeconds;

/// Every fixture happens at this instant (2026-09-21T10:13:20Z). A fixture's clock is a
/// `base_time::ManualClock` made at it.
pub const NOW: UnixSeconds = UnixSeconds(1_790_000_000);
