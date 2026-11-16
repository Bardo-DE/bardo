//! The XDG roots from the environment: read once, in `main`, and injected everywhere else.

use base_xdg::{PathError, Xdg};
use memory_core::Dirs;
use std::path::PathBuf;

/// Why the directories could not be found.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum XdgError {
    /// Neither an `XDG_*` variable nor `HOME` names an absolute path.
    #[error(transparent)]
    NoHome(#[from] PathError),
    /// `XDG_RUNTIME_DIR` is not set (memoryd needs a runtime directory for edit copies).
    #[error("XDG_RUNTIME_DIR is not set")]
    NoRuntimeDir,
}

/// The four roots, as `lookup` says: data, cache, config and runtime.
pub(crate) fn roots(lookup: impl Fn(&str) -> Option<String>) -> Result<[PathBuf; 4], XdgError> {
    let xdg = Xdg::new(lookup);
    Ok([
        xdg.data_home()?,
        xdg.cache_home()?,
        xdg.config_home()?,
        xdg.runtime_dir().ok_or(XdgError::NoRuntimeDir)?,
    ])
}

/// The directories as `lookup` (the environment, or a test's map) says: `XDG_DATA_HOME`
/// (default `~/.local/share`), `XDG_CACHE_HOME` (`~/.cache`), `XDG_CONFIG_HOME` (`~/.config`)
/// and the required `XDG_RUNTIME_DIR`.
pub fn dirs_from(lookup: impl Fn(&str) -> Option<String>) -> Result<Dirs, XdgError> {
    let [data, cache, config, runtime] = roots(lookup)?;
    Ok(Dirs::new(data, cache, config, runtime))
}
