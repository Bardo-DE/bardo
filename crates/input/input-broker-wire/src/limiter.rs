//! The per-device rate limit on `Events` datagrams, pure: the caller passes the time.

use crate::limits::{EVENT_BURST, EVENT_DATAGRAMS_PER_SECOND, FLOOD_CUT_MS, FLOOD_GAP_MS};
use base_time::Mono;

/// Whole milliseconds from `earlier` to `later`, zero if `earlier` is later.
#[must_use]
pub fn millis_since(later: Mono, earlier: Mono) -> u64 {
    u64::try_from(later.since(earlier).as_millis()).unwrap_or(u64::MAX)
}

/// What to do with one datagram.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Admit {
    /// Write it.
    Pass,
    /// Over the rate: drop it and count it.
    Drop,
    /// The excess has lasted [`FLOOD_CUT_MS`]: close the connection.
    Cut,
}

/// A token bucket of [`EVENT_BURST`] datagrams refilled at [`EVENT_DATAGRAMS_PER_SECOND`], plus the
/// count of how long the excess has gone on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Limiter {
    /// Thousandths of a datagram, so the refill stays in whole numbers.
    tokens: u64,
    at: Mono,
    /// When the current excess began and when it was last seen; `None` when there is none.
    flood: Option<(Mono, Mono)>,
}

const MILLI: u64 = 1_000;

impl Limiter {
    /// A full bucket at `now`.
    #[must_use]
    pub const fn new(now: Mono) -> Self {
        Self {
            tokens: EVENT_BURST * MILLI,
            at: now,
            flood: None,
        }
    }

    /// Decides one datagram arriving at `now`.
    pub fn admit(&mut self, now: Mono) -> Admit {
        let refill = millis_since(now, self.at).saturating_mul(EVENT_DATAGRAMS_PER_SECOND);
        self.tokens = self.tokens.saturating_add(refill).min(EVENT_BURST * MILLI);
        self.at = now;
        match self.tokens.checked_sub(MILLI) {
            Some(left) => {
                self.tokens = left;
                Admit::Pass
            }
            None => self.over(now),
        }
    }

    fn over(&mut self, now: Mono) -> Admit {
        let start = match self.flood {
            Some((start, last)) if millis_since(now, last) <= FLOOD_GAP_MS => start,
            _ => now,
        };
        self.flood = Some((start, now));
        if millis_since(now, start) >= FLOOD_CUT_MS {
            Admit::Cut
        } else {
            Admit::Drop
        }
    }
}
