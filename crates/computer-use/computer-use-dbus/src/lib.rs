//! cuad's D-Bus API: `org.bardo.ComputerUse1.Manager` at `/org/bardo/ComputerUse1` and `org.bardo.ComputerUse1.Run` at
//! `/org/bardo/ComputerUse1/run/<id>`. Each interface is declared twice: a proxy trait for callers (the shell,
//! companiond, intentd) and a skeleton for cuad, whose introspection is the checked-in
//! `dbus/org.bardo.ComputerUse1.xml` (see `tests/it/introspection.rs`). Bodies are the JSON of the named
//! `computer-use-bus` type in an `s` argument, in the `CuaEnvelope` where a body is sent; the caller's
//! identity is derived from the connection and never sent. Signatures only: every skeleton
//! method answers `NotSupported`.
//!
//! A run's result is not a member of this interface: it is a `provenance::Message` of kind `Report`
//! delivered through the router, and the `Finished` signal says which.

mod error;
mod introspect;
mod manager;
mod names;
mod run;

pub use error::CuaError;
pub use introspect::{FILE_NAME, introspection};
pub use manager::{ManagerProxy, ManagerSkeleton};
pub use names::{CUA_BUS, CUA_PATH, PROVIDER_BUS, run_path};
pub use run::{RunProxy, RunSkeleton};
