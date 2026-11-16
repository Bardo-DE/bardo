//! Reading the config files memory keeps. Writing them is `base_fs::atomic::AtomicWrite::PRIVATE`.

use base_fs::{ReadCap, ReadError, read_text_capped};
use std::path::Path;

/// The most a config file may hold.
pub const CONFIG_CAP: ReadCap = ReadCap::mib(4);

/// The text of a config file: `None` when there is no such file (a fresh install), an error for
/// any other failure to read it, so a file that cannot be read is never taken for an empty one.
pub fn read_config(path: &Path) -> Result<Option<String>, ReadError> {
    match read_text_capped(path, CONFIG_CAP) {
        Ok(text) => Ok(Some(text)),
        Err(ReadError::NotFound { .. }) => Ok(None),
        Err(e) => Err(e),
    }
}
