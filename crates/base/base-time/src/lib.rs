//! The time, as a seam: the one [`Clock`] trait every crate that dates something takes, the
//! system's wall clock, a fixed clock for tests, the monotonic seam for what is paced rather than
//! dated, and [`UnixSeconds`], the instant on the wire and in stores. Nothing here reads the time
//! unless a caller asks [`SystemClock`] or [`SystemMonotonic`] for it.

mod clock;
mod unix;

pub use clock::{Clock, FixedClock, ManualMonotonic, Monotonic, SystemClock, SystemMonotonic};
pub use unix::UnixSeconds;
