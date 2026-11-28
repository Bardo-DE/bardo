//! Reading desktop entries from disk (effects: the filesystem).

use std::path::PathBuf;

use shell_desktop_fs::icons::{IconRoots, IconTheme};
use shell_model::apps::App;
use xdg_core::EntryLocale;
use xdg_fs::load_entries;

use super::entry::parse_entry;

/// Every desktop entry under `dirs`, in precedence order with each id once.
///
/// Every entry under `dirs` (the applications folders, [`Dirs::applications`](shell_model::dirs::Dirs)),
/// in precedence order with each id once, icons resolved in `theme` (an absolute `Icon=` only
/// inside `roots`), with the desktop's current locales. A folder that does not exist, or a
/// `.desktop` file that cannot be read (or is over [`xdg_core::DESKTOP_ENTRY_CAP`], or is not text), is
/// skipped rather than failing the scan. The walk, the ids and the shadowing are
/// [`xdg_fs::load_entries`]'s, the same for every part of the desktop.
#[must_use]
pub fn scan(
    dirs: &[PathBuf],
    theme: &IconTheme,
    roots: &IconRoots,
    locales: &[EntryLocale],
) -> Vec<App> {
    load_entries(dirs)
        .iter()
        .filter_map(|entry| parse_entry(entry, theme, roots, locales))
        .collect()
}
