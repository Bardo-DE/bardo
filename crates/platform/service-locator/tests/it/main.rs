//! The service locator's integration tests, as one binary.
//!
//! Each former `tests/*.rs` file is a module here, so the crate links once instead of once per
//! file. `helpers` holds what the modules share. Nothing here needs its own process: no test sets
//! process-global state, forks, or depends on a clean environment, and the cross-process tests
//! only set env on the children they spawn.

mod across_processes;
mod address;
mod helpers;
mod lifecycle;
