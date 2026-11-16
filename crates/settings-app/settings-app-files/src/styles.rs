//! The style files the person wrote: the desktop-wide `bardo/style.css` and each app's own
//! `bardo/<app>/style.css` (a program outside the desktop: `<app>/style.css`) under the settings
//! folder. Only the text is read here; what it says about the styles is the design system's lint.

use crate::read::text_of;
use crate::values::{ConfigHome, DataHome};
use settings_app_model::StyleOwner;
use std::path::{Path, PathBuf};

/// The file every style sheet is called.
const SHEET: &str = "style.css";

/// The folder of the desktop-wide sheet, which is also the folder of every program of the
/// desktop (`bardo/shell`, `bardo/files`).
const DESKTOP_FOLDER: &str = "bardo";

/// The most app folders looked into: a settings folder holds a few dozen, and a runaway one must
/// not hold the window.
const MOST_FOLDERS: usize = 512;

/// One style file that could be read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StyleSheet {
    /// Whose it is.
    pub owner: StyleOwner,
    /// Its text.
    pub css: String,
}

fn sheet_in(folder: &Path, owner: StyleOwner) -> Option<StyleSheet> {
    let css = text_of(&folder.join(SHEET)).ok().flatten()?;
    Some(StyleSheet { owner, css })
}

/// The names of the folders inside `parent`, without `skip`, in the order of their names.
fn folders_in(parent: &Path, skip: Option<&str>) -> Vec<String> {
    let mut folders: Vec<String> = std::fs::read_dir(parent)
        .into_iter()
        .flatten()
        .flatten()
        .filter(|entry| entry.path().is_dir())
        .filter_map(|entry| entry.file_name().into_string().ok())
        .filter(|name| Some(name.as_str()) != skip)
        .take(MOST_FOLDERS)
        .collect();
    folders.sort();
    folders
}

/// The style files under `home` that exist and can be read: the desktop-wide one first, then each
/// program of the desktop (a folder inside `bardo`), then each program outside it (a folder of
/// `home`), each in the order of their names. A file that is missing or cannot be read is left
/// out, as is a folder that cannot be listed.
pub fn read_style_sheets(home: &ConfigHome) -> Vec<StyleSheet> {
    let level = home.0.join(DESKTOP_FOLDER);
    let desktop = sheet_in(&level, StyleOwner::Desktop);
    let own = folders_in(&level, None).into_iter().filter_map(|name| {
        let at = level.join(&name);
        sheet_in(&at, StyleOwner::App(name))
    });
    let foreign = folders_in(&home.0, Some(DESKTOP_FOLDER))
        .into_iter()
        .filter_map(|name| {
            let at = home.0.join(&name);
            sheet_in(&at, StyleOwner::App(name))
        });
    desktop.into_iter().chain(own).chain(foreign).collect()
}

/// The classes each installed app publishes for a style to rely on, one list per app: what the
/// apps installed under `system` data directories and under `data` (the person's) put in their
/// `bardo/selectors` folders. A style that uses one of these is not using an internal selector.
/// The directories are named by the caller, so a test reads only its own.
pub fn published_classes(data: &DataHome, system: &[PathBuf]) -> Vec<Vec<String>> {
    let dirs: Vec<PathBuf> = system
        .iter()
        .chain(std::iter::once(&data.0))
        .map(|root| root.join("bardo").join("selectors"))
        .collect();
    ui_settings::discover_parts(&dirs)
        .into_iter()
        .map(|parts| parts.classes)
        .collect()
}
