//! The crate's integration tests: one executable, one module per topic.
//! Modules that need a feature carry it here; the ones that start the daemon build only on Linux,
//! where the daemon's dev-dependencies are (Cargo.toml).

#[cfg(all(target_os = "linux", feature = "dbus"))]
mod common;
mod dbus_guests;
mod dbus_open;
mod dbus_served;
mod in_process_session;
mod prepare;
mod session_shape;
#[cfg(feature = "lending")]
mod tailnet_lending;
