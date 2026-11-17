//! The chord keymap: a portable keymap core.
//!
//! Apps declare their own actions with a portable default chord and handle actions; they never
//! read modifiers or decide Cmd versus Ctrl. A [`Keymap`] holds, for one [`Platform`], the
//! platform's conventions, the system's own shortcut settings, the person's changes and the apps'
//! registered actions, and turns a toolkit's [`KeyInput`] into an [`Action`].
//!
//! The crate is pure: no I/O, no async. Sources that read a system's settings live in other
//! crates and implement [`KeymapSource`].
//!
//! Keys and modifier flags are [`keyboard_types`]' own, re-exported here so a toolkit hands in
//! the same version the chord keymap reads; a [`Chord`] narrows them to its canonical, `Copy` form.

pub use keyboard_types;

mod action;
mod chord;
mod chord_text;
mod convention;
mod detect;
mod key;
mod key_input;
pub mod keyfile;
mod keymap;
mod modifier;
mod overrides;
mod platform;
mod reach;
mod reserved;
#[cfg(feature = "serde")]
mod serde_support;
mod source;
mod standard_action;

pub use action::{Action, ActionIdError, AppAction, AppId};
pub use chord::{Chord, DefaultChord};
pub use chord_text::ChordError;
pub use convention::{Convention, ConventionTable};
pub use detect::{HostOs, OVERRIDE_VAR, detect, detect_process};
pub use key::Key;
pub use key_input::KeyInput;
pub use keymap::{ConflictError, Keymap, Registration};
pub use modifier::CHORD_MODIFIERS;
pub use overrides::{OverrideEntry, OverrideProblem, Overrides};
pub use platform::{Context, Desktop, Platform, PlatformError};
pub use reach::Reach;
pub use reserved::reserved_reason;
pub use source::{
    ChangeCallback, ConventionSource, KeymapSource, SourceError, SystemBinding, SystemImport, Watch,
};
pub use standard_action::{SpaceNumber, StandardAction};
