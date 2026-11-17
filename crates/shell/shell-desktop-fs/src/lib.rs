//! The shell's plain file helpers over the freedesktop folders: the home Trash ([`trash`]), the
//! autostart entries ([`autostart`]) and the icon theme lookup ([`icons`]).
//!
//! Files only: no bus, no runtime. Both the services that scan the desktop and the surfaces that
//! show it use these, so they live below both.

pub mod autostart;
mod error;
mod icon_roots;
pub mod icons;
pub mod trash;

pub use error::FileError;
