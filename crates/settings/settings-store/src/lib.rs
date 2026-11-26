//! Settings files as pure functions of text: what a settings document is ([`SettingsDoc`]), how
//! a text reads into it leniently (a bad value costs only its own key: [`decode`], [`Loaded`]),
//! what a save lays over the file already there ([`merged`]: unknown keys and comments stay), how
//! a key is edited in a file's text ([`apply_edits`]), the table-driven reader of a daemon's own
//! settings file ([`rules`]) and the latest-value channel a watch publishes on ([`latest`]).
//!
//! No file, no watcher, no bus and no runtime: reading and writing files is
//! `settings-store-fs`.
#![expect(
    missing_docs,
    reason = "lints baseline: this crate documents its public items when its domain is refactored (plan phases 3-7)"
)]

mod codec;
mod doc;
mod edit;
pub mod latest;
mod lenient;
mod preserve;
pub mod rules;

pub use codec::{decode, merged, understood};
pub use doc::{EncodeError, FileName, Format, SettingsDoc};
pub use edit::{Edit, EditError, apply_edits};
pub use lenient::{
    ClampedKey, InvalidKey, LimitBounds, Limits, Loaded, Read, UnknownKey, read, read_within,
};
pub use preserve::InTheWayError;
