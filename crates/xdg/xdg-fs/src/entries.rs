//! Desktop entries read from disk.

use crate::walk::{first_of_each_id, walk_applications};
use base_fs::read_text_capped;
use std::path::PathBuf;
use xdg_core::{ApplicationEntry, DESKTOP_ENTRY_CAP, DesktopFile};

/// Every entry under `dirs` (the `applications` folders, most important first), each id once, the
/// most important folder's file standing for it.
///
/// A file that cannot be read, is over [`DESKTOP_ENTRY_CAP`], is not text, or has no `[Desktop
/// Entry]` group is skipped.
#[must_use]
pub fn load_entries(dirs: &[PathBuf]) -> Vec<ApplicationEntry> {
    first_of_each_id(walk_applications(dirs))
        .into_iter()
        .filter_map(|found| {
            let text = read_text_capped(&found.path, DESKTOP_ENTRY_CAP).ok()?;
            let file = DesktopFile::parse(&text, DESKTOP_ENTRY_CAP).ok()?;
            ApplicationEntry::new(found.path, found.id, file)
        })
        .collect()
}
