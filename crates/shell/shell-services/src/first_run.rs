//! Whether a store was ever written: the one fact a seed (`shell_model::seed`) is decided by.

use std::path::Path;

use shell_model::seed::Store;

/// Whether `file` exists.
pub fn store_of(file: &Path) -> Store {
    if file.exists() {
        Store::Present
    } else {
        Store::Missing
    }
}
