//! The time, as a seam: the one [`Clock`] trait every crate that dates something takes, the
//! system's wall clock, a fixed clock for tests, the monotonic seam for what is paced rather than
//! dated, and [`UnixSeconds`], the instant on the wire and in stores. Nothing here reads the time
//! unless a caller asks [`SystemClock`] or [`SystemMonotonic`] for it. [`timeout::within`] bounds
//! a future by wall-clock time under any executor, and [`deadline`] names how long a call to another
//! process may take.

mod clock;
pub mod deadline;
pub mod timeout;
mod unix;

pub use clock::{Clock, FixedClock, ManualMonotonic, Monotonic, SystemClock, SystemMonotonic};
pub use unix::UnixSeconds;
