//! The compositor's D-Bus servers (C55): `org.bardo.Compositor1` (P2-Z) and, under feature
//! `debug-inject`, `org.bardo.Compositor.Debug1`, which `bardo-compositorctl debug` drives on the jail's
//! private bus. The inject and wait grammars are pure and always built, so their tables run
//! in every gate. `shell_calls` is the seam bindings call the shell through (A4); `space_registry` is
//! the seam the Spaces the compositor mints are registered with the account service through.
//! `Compositor1` proves each caller through the caller-identity library before the event loop
//! hears of the call; `privacy` serves the Privacy list for Settings; `a11y` checks that a
//! window's accessibility root belongs to the window's own program.

mod error;
mod handles;
mod inject;
mod shot;
mod stop_on_drop;
mod wait;

#[cfg(any(test, feature = "testing"))]
pub mod test_bus;
#[cfg(test)]
mod test_served;

pub mod a11y;
pub mod dbus;
pub mod privacy;
pub mod shell_calls;
pub mod signals;
pub mod sleep;
pub mod space_registry;
pub mod system_scheme;

pub use error::IpcError;
pub use handles::IpcHandles;
pub use inject::{DeviceVerb, InjectLine, InjectOrigin, parse_inject};
pub use shot::{ShotArea, parse_shot};
pub use wait::{WaitPredicate, parse_wait};
