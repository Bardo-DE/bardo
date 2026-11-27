//! Dev-only helpers for the accept suites (`cargo xtask accept`): where the binaries xtask built
//! beforehand are ([`bin`]), the argument list of `packaging/harness/jail.sh` ([`jail::Plan`]) and
//! a headless compositor on a private bus ([`Compositor`]). It depends on nothing of ours but
//! `base-testkit`; a test crate takes it as a dev dependency.

mod bins;
mod compositor;
pub mod jail;

pub use bins::{bin, bin_var};
pub use compositor::{Clock, Compositor, Options};
