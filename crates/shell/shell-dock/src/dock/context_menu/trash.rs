//! The Trash tile's rows (design/10 §10.3.9): Open, in the file manager (left out when none is
//! installed), and Empty Trash, which is off while the Trash holds nothing.

use shell_launcher::AppId;
use ui_components::prelude::Availability;

use super::DockMenuRow;

/// The Trash tile's rows; `files` is the file manager Open opens it in, and `empty_trash` says
/// whether Empty Trash can be chosen.
pub fn rows(files: Option<&AppId>, empty_trash: Availability) -> Vec<DockMenuRow> {
    files
        .cloned()
        .map(DockMenuRow::OpenTrash)
        .into_iter()
        .chain([DockMenuRow::EmptyTrash(empty_trash)])
        .collect()
}

#[cfg(test)]
#[path = "trash_tests.rs"]
mod tests;
