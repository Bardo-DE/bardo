//! The desktop's own compositor as `desktop_wl`'s Bardo arm.
//!
//! Apps, windows and Spaces from `bardo_desktop_v1` are mapped into the shell's
//! [`shell_model::desktop::DesktopView`], the shell's commands into Bardo requests, the shell
//! protocol's notes into the shell's words, and the connection the compositor hands the shell.
//!
//! The pieces, lowest first: `handout` (the seam that yields the handed connection),
//! `surfaces` (which registered surface plays which role), `view` and `command` (pure
//! mappings), `notes` (pure), `arm` (a [`arm::BardoArm`] over any `desktop_client::Desktop`:
//! a fake in tests, the wire client on the shell host's connection) and `shell` (the thread
//! that runs the arm on that connection).

mod arm;
mod command;
mod handout;
mod notes;
mod pictures;
mod queue_wake;
mod shell;
mod surfaces;
mod view;

/// The host's surface type, for a caller that registers one without naming the Wayland crate.
pub use wayland_client::protocol::wl_surface::WlSurface;

pub use arm::{ArmError, BardoArm, NoteSink};
pub use command::bardo_wire_of;
pub use handout::{Handout, HandoutError, ZbusHandout, shell_connection};
pub use notes::note_of;
pub use pictures::picture_of;
pub use shell::{ShellArm, ShellCall, ShellGoneError, SpawnError, spawn_shell_arm};
pub use surfaces::ShellSurfaces;
pub use view::bardo_view;
