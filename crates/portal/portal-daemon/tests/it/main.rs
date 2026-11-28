//! bardo-portal's tests: one binary. Everything here runs on fakes; nothing opens a `PipeWire` daemon or
//! a compositor, and the one module that needs a bus (`shell_bus`) starts a private one and fails, saying how to
//! install it, without a `dbus-daemon`. The jailed chain's checks live in `dev/accept/`.

mod app_proof;
mod chooser;
mod dev_callers;
mod grants;
mod guard;
mod inhibit;
mod names;
mod options;
mod peers;
mod remote_desktop;
mod screenshot;
mod session;
mod settings;
mod shell_bus;
mod shell_check;
