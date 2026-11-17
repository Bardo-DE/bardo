//! A program's end of the key remap daemon.
//!
//! - [`Connection`]: the socket, one message per packet, `Hello` checked.
//! - [`KeyRemapSource`]: this desktop's `keymap::KeymapSource`. It lays the chord keymap's conventions
//!   for our desktop, the desktop's own bindings ([`system_bindings`]) and the person's keymap
//!   over each other, and tells the program when the person's keymap changes. Without a
//!   daemon it reads the keymap file itself, so a program still honours the person's keymap.
//! - [`Registered`]: an app's actions told to the key remap daemon so Settings can list them. Informational:
//!   resolution stays in each app, through keymap.
//!
//! No keymap noun is defined here; every action, chord and keymap is the chord keymap's.

mod bounded;
mod connection;
mod fallback;
mod fetch;
mod register;
mod source;
mod system;
#[cfg(test)]
mod testing;
mod watch;

pub use bounded::{FileText, read_text};
pub use connection::{ClientError, Connection, Heard};
pub use key_remap_proto::{Registration, SOCKET_PATH};
pub use register::Registered;
pub use source::{KeyRemapSource, Layer, Origin};
pub use system::system_bindings;
