//! What runs an engine (io: tokio): the loop around the pure scheduler and the hub it publishes to.
//!
//! - `driver`: [`Driver`] runs one engine on the scheduler's wake-ups: asks what is next, sleeps
//!   until it, runs the cycle, publishes the status.
//! - `hub`: what syncd shares between the engines and whatever serves them (the bus object): each
//!   running dataset's name, who may see it, its latest status and its pause switch.
//! - `removal`: [`removal::wipe`], what removing an account deletes of syncd's.
//! - `resolve`: the requests that go from the hub to a driver (settle a conflict, confirm a
//!   discard) and the words their refusals have.
//! - `clock`: [`clock::Wall`], the system clock that can run faster for a test.

pub mod clock;
pub mod driver;
pub mod hub;
pub mod removal;
pub mod resolve;

pub use driver::Driver;
