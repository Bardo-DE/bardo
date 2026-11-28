//! What the socket thread allows per pass, so no subscriber can keep the others, or the reader's
//! messages, waiting. (The cap on subscribers, and when a refusal is logged, are decisions: they are
//! in `mouse_gestures_core::socket`.)

use std::time::Duration;

/// Packets read from one subscriber in one pass; the rest wait for the next pass, so one chatty
/// subscriber cannot keep the others, or the reader's messages, waiting.
pub(super) const PACKETS_PER_PASS: usize = 16;
/// Connections accepted in one pass.
pub(super) const ACCEPTS_PER_PASS: usize = 8;
/// Messages the reader may have queued for the socket thread. At 90 reports a second this is
/// more than ten seconds of a stuck thread; past it the reader drops messages rather than wait.
pub(super) const QUEUE_MAX: usize = 1024;
/// How often the seat file is read when no subscriber says anything, so a user switch is noticed
/// with no traffic.
pub(super) const SEAT_RECHECK: Duration = Duration::from_secs(1);
