//! The daemon: `bardo-shell run` ([`run`]) takes the single-instance lock, starts the services, the
//! IPC server and the settings watch, connects the host and keeps one row of surfaces per output.
//!
//! `events` handles what the host reports, `compositor` holds the desktop's own compositor's
//! connection, arm and notes, `surfaces` spawns what an output lacks (`spawn` any `Surface`),
//! `start` starts the services, `services` instructs them, `settings` holds the settings and `msg` is the
//! message other threads send the host.

mod compositor;
mod events;
mod hardening;
mod mouse_gestures;
pub mod msg;
mod run;
pub(crate) mod services;
pub(crate) mod settings;
pub(crate) mod spawn;
pub(crate) mod start;
pub(crate) mod surfaces;
mod targets;

pub use run::run;
