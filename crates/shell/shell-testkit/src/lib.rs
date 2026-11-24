//! Dev-only helpers shared by the shell crates' tests: signal listeners on a private bus, and (in
//! `examples/`) the stand-in programs tests and scripts run. The private bus and the scratch
//! folders are `base-testkit`'s.

mod signals;

pub use signals::{Heard, Signals};
