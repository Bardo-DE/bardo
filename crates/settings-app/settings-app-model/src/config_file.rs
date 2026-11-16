//! A settings file's place under the config home, checked once where the schema is read.

use crate::warning::StaleReason;
use std::path::{Component, Path, PathBuf};
use ui_settings::schema::FilePath;

/// `<app-dir>/<name>.toml`, relative to `$XDG_CONFIG_HOME`: exactly two plain components, so a
/// schema can never name a path outside the config home.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ConfigFile(PathBuf);

impl ConfigFile {
    /// `file` as a config file, or why it is refused.
    pub fn parse(file: &FilePath) -> Result<ConfigFile, StaleReason> {
        let path = Path::new(&file.0);
        let plain = path
            .components()
            .all(|part| matches!(part, Component::Normal(_)));
        let shaped =
            path.components().count() == 2 && path.extension().is_some_and(|ext| ext == "toml");
        if plain && shaped {
            Ok(ConfigFile(path.to_path_buf()))
        } else {
            Err(StaleReason::UnsafeFile(file.0.clone()))
        }
    }

    /// The path under the config home.
    pub fn relative(&self) -> &Path {
        &self.0
    }
}

#[cfg(test)]
mod tests {
    use super::ConfigFile;
    use ui_settings::schema::FilePath;

    const CASES: &[(&str, bool)] = &[
        ("shell/settings.toml", true),
        ("bardo/appearance.toml", true),
        ("../settings.toml", false),
        ("/etc/settings.toml", false),
        ("shell/../../x.toml", false),
        ("settings.toml", false),
        ("shell/deep/settings.toml", false),
        ("shell/settings.json", false),
        ("", false),
    ];

    #[test]
    fn only_app_dir_and_toml_name_is_a_config_file() {
        for (file, want) in CASES {
            let got = ConfigFile::parse(&FilePath((*file).to_owned())).is_ok();
            assert_eq!(got, *want, "{file:?}");
        }
    }
}
