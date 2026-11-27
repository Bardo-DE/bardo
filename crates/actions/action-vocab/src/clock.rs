//! The one clock a host plugs in: what tells the time, and waits it out. The system's is
//! `base_clock::SystemClock`, a test's is in `action-fake`; nothing else in the repo reads the
//! time or sleeps for a deadline.

/// The clock a host plugs in: [`base_time::Clock`] and [`base_time::Sleeper`] together.
///
/// The clock says now, as Unix seconds; the sleeper is [`base_time::Sleeper`] (the deadline a host races a peer against: the real clock sleeps, a
/// fake completes at once only for no time at all). Every type with both is one.
pub trait Clock: base_time::Clock + base_time::Sleeper {}

impl<T: base_time::Clock + base_time::Sleeper> Clock for T {}
