//! A settings document's text, as a pure function: what a text reads as, whether it is the
//! document at all, and what a save writes over the file already there.

use crate::doc::{Format, SettingsDoc};
use crate::lenient::{Loaded, Read, read, read_within};
use crate::preserve::{self, InTheWay};

/// `text` read as a `D`; text that is not the document's format at all is the defaults with the
/// whole file reported invalid.
#[must_use]
pub fn decode<D: SettingsDoc>(text: &str) -> Loaded<D> {
    match read_within::<D>(text, D::FORMAT, &D::limits()) {
        Read::Loaded(loaded) => loaded,
        Read::Garbled { reason } => Loaded::garbled(D::default(), reason),
    }
}

/// `Ok` when `text` is a `D`'s file (a bad value costs its own key, so it still is), else why not.
///
/// # Errors
///
/// Returns why `text` is not the document's format at all.
pub fn understood<D: SettingsDoc>(text: &str) -> Result<(), String> {
    match read::<D>(text, D::FORMAT) {
        Read::Garbled { reason } => Err(reason),
        Read::Loaded(_) => Ok(()),
    }
}

/// The text a save writes: `body` laid over the `existing` TOML file, or `body` itself when there
/// is no file or the format has no comments to keep.
///
/// # Errors
///
/// Returns [`InTheWay`] when the file is not a `D`.
pub fn merged<D: SettingsDoc>(existing: Option<String>, body: &str) -> Result<String, InTheWay> {
    let Some(text) = existing else {
        return Ok(body.to_owned());
    };
    match read::<D>(&text, D::FORMAT) {
        Read::Garbled { .. } => Err(InTheWay(None)),
        Read::Loaded(loaded) => match D::FORMAT {
            Format::Toml => preserve::merge(&text, body, &loaded.unknown),
            _ => Ok(body.to_owned()),
        },
    }
}
