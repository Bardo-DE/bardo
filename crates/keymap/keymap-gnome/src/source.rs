//! `GnomeSource`: a dconf dump from an injected path, over GNOME's conventions.

use std::path::PathBuf;

use base_fs::{ReadCap, ReadError, read_text_capped};
use keymap::{Keymap, KeymapSource, Platform, SourceError};

use crate::parse::parse_dconf_dump;

/// The most of a desktop's shortcut settings file that is read: 4 MiB, far above any real one
/// (kdeglobals and a dconf dump are tens of KiB) and small enough to hold in memory at once.
const SETTINGS_FILE_LIMIT: ReadCap = ReadCap::mib(4);

/// Reads the person's GNOME shortcut choices from a dconf keyfile dump (the output of
/// `dconf dump /`, saved by the caller). A missing file means nothing was changed, so the keymap
/// is GNOME's conventions as they are.
#[derive(Debug, Clone)]
pub struct GnomeSource {
    path: PathBuf,
}

impl GnomeSource {
    /// A source reading the dump at `path`.
    pub fn new(path: impl Into<PathBuf>) -> GnomeSource {
        GnomeSource { path: path.into() }
    }
}

impl KeymapSource for GnomeSource {
    fn load(&self, platform: Platform) -> Result<Keymap, SourceError> {
        let text = match read_text_capped(&self.path, SETTINGS_FILE_LIMIT) {
            Ok(text) => text,
            Err(ReadError::NotFound { .. }) => String::new(),
            Err(error) => {
                return Err(SourceError::Read {
                    place: self.path.display().to_string(),
                    reason: error.to_string(),
                });
            }
        };
        Ok(Keymap::conventional(platform).with_system(parse_dconf_dump(&text).bindings))
    }
}
