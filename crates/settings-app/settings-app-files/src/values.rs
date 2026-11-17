//! The values of one settings file.

use settings_app_model::ConfigFile;
use settings_types::schema::KeyPath;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// `$XDG_CONFIG_HOME` (or a scratch directory standing in for it): the directory the settings
/// files' `bardo/<app>/<name>.toml` paths are under.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfigHome(pub PathBuf);

impl ConfigHome {
    /// Where `file` is on disk.
    #[must_use]
    pub fn path_of(&self, file: &ConfigFile) -> PathBuf {
        self.path_of_relative(file.relative())
    }

    /// Where the file `<app>/<name>` of a program is on disk: the program's folder
    /// (`settings_types::app_folder`) with the name in it. A path of any other shape is under the
    /// config home as it stands.
    #[must_use]
    pub fn path_of_relative(&self, relative: &Path) -> PathBuf {
        let mut parts = relative.iter();
        let app = parts.next().and_then(|app| app.to_str());
        let rest: PathBuf = parts.collect();
        match app {
            Some(app) if !rest.as_os_str().is_empty() => {
                settings_types::app_folder(&self.0, app).join(rest)
            }
            _ => self.0.join(relative),
        }
    }
}

/// `$XDG_DATA_HOME` (or a scratch directory standing in for it): where the actions service keeps the agents it
/// installed and what each last offered.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DataHome(pub PathBuf);

/// `$XDG_CACHE_HOME` (or a scratch directory standing in for it): where the small pictures of the
/// Wallpaper page are kept, in the shared `thumbnails` folder every file manager reads.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CacheHome(pub PathBuf);

/// The current value of every key of a file that has one: what the file says where the key's
/// control can hold it, and the key's default elsewhere.
///
/// A value the control cannot hold (a gesture bound to a key chord, an out-of-range number, an
/// unknown word) is kept apart as it was written: the window shows it read-only, and nothing
/// ever rewrites it.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct FileValues {
    values: BTreeMap<KeyPath, toml::Value>,
    as_written: BTreeMap<KeyPath, toml::Value>,
}

impl FileValues {
    /// The value of `path`.
    #[must_use]
    pub fn get(&self, path: &KeyPath) -> Option<&toml::Value> {
        self.values.get(path)
    }

    /// The value the file holds at `path` when no control can represent it.
    #[must_use]
    pub fn as_written(&self, path: &KeyPath) -> Option<&toml::Value> {
        self.as_written.get(path)
    }

    /// These values with `path` set to `value`, which the person chose, so it is representable.
    #[must_use]
    pub fn with(mut self, path: &KeyPath, value: toml::Value) -> FileValues {
        self.as_written.remove(path);
        self.values.insert(path.clone(), value);
        self
    }

    pub(crate) fn from_parts(
        values: BTreeMap<KeyPath, toml::Value>,
        as_written: BTreeMap<KeyPath, toml::Value>,
    ) -> FileValues {
        FileValues { values, as_written }
    }
}
