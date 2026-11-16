//! desktop-client's integration tests: the reducers, pairing, `supports`, configuration
//! validation, frame conversion and the fake.
//!
//! One integration binary: every test file is a module here, and `support` is shared. The wire
//! layer needs a real compositor and has no test here; it is kept thin so the reducers carry
//! the logic.

mod active_link;
mod backend;
mod capture;
#[cfg(feature = "fake")]
mod fake;
mod frame;
mod output;
mod pairing;
mod protocol;
mod quire;
mod support;
mod toplevel;
mod workspace;
