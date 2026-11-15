//! The crate's integration tests: one executable, one module per topic.
//! A separate test target needs a stated reason.

mod absent_bus;
mod handover;
mod modules_bus;
mod modules_fake;
mod private_bus;
mod spaces_bus;
mod spaces_fakes;
mod spaces_owner;
