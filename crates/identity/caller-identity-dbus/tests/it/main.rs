//! The integration tests of caller-identity-dbus: a private bus per test, a fixture tree for `/proc`, a
//! stand-in unit manager. Nothing reaches the real session.

mod bus;
mod manager;
mod owner;
mod peers;
