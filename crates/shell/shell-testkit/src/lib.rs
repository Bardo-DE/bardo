//! Dev-only helpers shared by the shell crates' tests: signal listeners on a private bus, and (in
//! `examples/`) the stand-in programs tests and scripts run. The private bus and the scratch
//! folders are `base-testkit`'s.
#![expect(
    missing_docs,
    reason = "lints baseline: this crate documents its public items when its domain is refactored (plan phases 3-7)"
)]

mod signals;

pub use signals::{Heard, Signals};
