//! compositor-testkit's integration tests: the fixture, the globals golden, adopt, the protocol wire tests and the renderer scenes.
//!
//! One integration binary: every former `tests/*.rs` is a module here, and `support` is shared.

mod adopt;
mod agent_lease;
mod bindings;
mod fixture;
mod fuzz;
mod globals;
mod input_convert;
mod logind;
mod protocol;
mod relax;
mod remote_eis;
mod scene;
mod space_sync;
mod topology;
mod tty;
mod writeback;
mod xwayland;
