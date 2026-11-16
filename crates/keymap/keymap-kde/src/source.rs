//! `KdeSource`: kdeglobals from an injected path, over KDE's conventions.

use std::path::PathBuf;

use base_fs::{ReadCap, ReadError, read_text_capped};
use keymap::{Keymap, KeymapSource, Platform, SourceError};

use crate::parse::parse_kdeglobals;

/// The most of a desktop's shortcut settings file that is read: 4 MiB, far above any real one
/// (kdeglobals and a dconf dump are tens of KiB) and small enough to hold in memory at once.
const SETTINGS_FILE_LIMIT: ReadCap = ReadCap::mib(4);

/// Reads the user's KDE shortcut choices from one kdeglobals file. A missing file means the
/// person changed nothing, so the keymap is KDE's conventions as they are.
#[derive(Debug, Clone)]
pub struct KdeSource {
    path: PathBuf,
}

impl KdeSource {
    /// A source reading `path` (usually `<config dir>/kdeglobals`).
    pub fn new(path: impl Into<PathBuf>) -> KdeSource {
        KdeSource { path: path.into() }
    }
}

impl KeymapSource for KdeSource {
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
        Ok(Keymap::conventional(platform).with_system(parse_kdeglobals(&text).bindings))
    }
}
