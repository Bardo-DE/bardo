//! The bearer token of an attached engine (vLLM's `--api-key`), read from its file at each connect.
//!
//! The file is the person's own and private to them: a file any other user could read is refused,
//! and so is one another user owns. The token is held in [`SecretText`] (whose `Debug` says nothing)
//! and goes nowhere but the `Authorization` header: no log line, error or `Debug` output of this
//! module holds it.

use base_secret::SecretText;
use inference_router::attached::key::{KeyFile, KeyFileProblem};
use std::io::Read;
use std::os::unix::fs::MetadataExt;

/// The longest key kept, in bytes: a token is a few dozen.
const LONGEST: u64 = 4096;

/// The token in `key`'s file now. It is opened once and looked at through that handle, so the
/// file that was checked is the file that is read; a symlink is followed to the file it names
/// (and that file is checked).
///
/// # Errors
/// `KeyFileProblem` when the file is missing, cannot be read, is not a regular file private to
/// the person, or is not usable text.
pub fn read_key(key: &KeyFile) -> Result<SecretText, KeyFileProblem> {
    let mut file = std::fs::File::open(key.path()).map_err(|e| match e.kind() {
        std::io::ErrorKind::NotFound => KeyFileProblem::Missing,
        kind => KeyFileProblem::Unreadable(kind),
    })?;
    let meta = file
        .metadata()
        .map_err(|e| KeyFileProblem::Unreadable(e.kind()))?;
    if !meta.is_file() {
        return Err(KeyFileProblem::NotAFile);
    }
    let mode = meta.mode() & 0o777;
    if mode & 0o077 != 0 {
        return Err(KeyFileProblem::NotPrivate { mode });
    }
    if meta.uid() != rustix::process::geteuid().as_raw() {
        return Err(KeyFileProblem::NotYours);
    }
    let mut bytes = Vec::new();
    file.by_ref()
        .take(LONGEST + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| KeyFileProblem::Unreadable(e.kind()))?;
    let text = String::from_utf8(bytes).map_err(KeyFileProblem::NotText)?;
    let token = text.trim();
    let fits = !token.is_empty()
        && u64::try_from(token.len()).is_ok_and(|len| len <= LONGEST)
        && token.chars().all(|c| c.is_ascii_graphic());
    if fits {
        Ok(SecretText::new(token))
    } else {
        Err(KeyFileProblem::Unusable)
    }
}

#[cfg(test)]
mod tests;
