//! The environment values that decide where desktop entries are.

use std::path::PathBuf;
use xdg_core::XdgEnv;

/// The values of this process's environment.
#[must_use]
pub fn process_xdg_env() -> XdgEnv {
    XdgEnv {
        data_home: std::env::var_os("XDG_DATA_HOME"),
        data_dirs: std::env::var_os("XDG_DATA_DIRS"),
        home: std::env::var_os("HOME").map(PathBuf::from),
    }
}
