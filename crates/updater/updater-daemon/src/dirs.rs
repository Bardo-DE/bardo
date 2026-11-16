//! Where the daemon keeps its own files, from the XDG variables.

use std::ffi::OsString;
use std::path::PathBuf;

/// The three places the update service reads and writes as a person.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Dirs {
    /// `$XDG_CONFIG_HOME/bardo/updater/config.toml`.
    pub config_file: PathBuf,
    /// `$XDG_STATE_HOME/bardo/updater/state.toml`.
    pub state_file: PathBuf,
    /// `$XDG_CACHE_HOME/bardo/updater`.
    pub cache_dir: PathBuf,
}

impl Dirs {
    /// Resolves the places from `var`, which looks up an environment variable (so tests need not
    /// touch the real environment). `None` when there is neither the XDG variable nor `HOME`.
    pub fn resolve(var: impl Fn(&str) -> Option<OsString>) -> Option<Self> {
        let under = |xdg: &str, fallback: &str| -> Option<PathBuf> {
            var(xdg)
                .filter(|v| !v.is_empty())
                .map(PathBuf::from)
                .filter(|p| p.is_absolute())
                .or_else(|| var("HOME").map(|h| PathBuf::from(h).join(fallback)))
        };
        Some(Self {
            config_file: under("XDG_CONFIG_HOME", ".config")?.join("bardo/updater/config.toml"),
            state_file: under("XDG_STATE_HOME", ".local/state")?.join("bardo/updater/state.toml"),
            cache_dir: under("XDG_CACHE_HOME", ".cache")?.join("bardo/updater"),
        })
    }
}

/// Temporary: delete with the old-folders crate in the release after everyone has started once.
///
/// Moves the pre-bardo `slipway` folders (config, state, cache) to the folders `dirs` names, before
/// anything is read. Each old folder sits beside the new `bardo` folder of its XDG base.
pub fn move_pre_bardo_folders(dirs: &Dirs) {
    let pairs = [
        (&dirs.config_file, true),
        (&dirs.state_file, true),
        (&dirs.cache_dir, false),
    ];
    for (path, is_file) in pairs {
        let new = if is_file {
            path.parent()
        } else {
            Some(path.as_path())
        };
        let Some(new) = new else { continue };
        // `<base>/bardo/updater` -> `<base>/bardo-updater`.
        let Some(base) = new.parent().and_then(std::path::Path::parent) else {
            continue;
        };
        let old = base.join("slipway");
        match old_folders::move_once(&old, new) {
            Ok(moved) if moved.moved > 0 => {
                eprintln!(
                    "bardo-updater: moved {} entries from {} to {}",
                    moved.moved,
                    old.display(),
                    new.display()
                );
            }
            Ok(_) => {}
            Err(error) => {
                eprintln!("bardo-updater: could not move {}: {error}", old.display());
            }
        }
    }
}
