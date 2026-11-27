//! The time, as a seam: the one [`Clock`] trait every crate that dates something takes, a fixed and
//! a manual clock for tests, the monotonic seam ([`Monotonic`], [`Mono`]) for what is paced
//! rather than dated, the [`Sleeper`] seam for waiting, and [`UnixSeconds`], the instant on the
//! wire and in stores. Nothing here reads the system's time or sleeps for real: the system's
//! clocks and sleeper are in `base-clock`. [`timeout::within`] bounds a future by wall-clock
//! time under any executor, and [`deadline`] names how long a call to another process may take.

mod clock;
pub mod deadline;
mod mono;
mod sleep;
pub mod timeout;
mod unix;

pub use clock::{Clock, FixedClock, ManualClock};
pub use mono::{ManualMonotonic, Mono, Monotonic};
pub use sleep::{ManualSleeper, SharedSleeper, Sleeper};
pub use unix::UnixSeconds;
