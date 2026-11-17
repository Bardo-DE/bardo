//! The compositor's own shell bindings, read from the desktop's keymap.
//!
//! The keymap is the chord keymap's layers: the conventions for our desktop, the machine's keymap, the
//! person's keymap, through key-remap-client's `KeyRemapSource`.
//!
//! - [`chord`]: a chord keymap chord as the compositor's own (pure).
//! - [`shell`]: which standard actions the compositor carries out, and the bindings a keymap
//!   gives them (pure).
//! - [`live`]: the source for a config home, the first load and the reload when the keymap
//!   changes.
//!
//! Chords stay data of the keymap; this module holds action names only (the chord keymap's
//! `StandardAction`), never raw keys. A `[bindings]` entry in the compositor's own settings document
//! comes first and so wins over the keymap for the same chord.

mod chord;
mod live;
mod shell;

pub use chord::chord_of;
pub use live::{attach, load_bindings, source_for};
pub use shell::{bindings_of, merge, shell_action};
