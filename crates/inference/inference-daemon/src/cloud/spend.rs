//! The spend ledger's file: where `inference_settings::Ledger` keeps its document (a small JSON
//! file, written whole through a rename, mode 0600) so a restart does not reset a cap.
//!
//! The counting and the caps are `inference-settings`'; this is the part that reads and writes the
//! file. A file that is there and cannot be read is not quietly forgotten (that would reset every
//! cap to nothing spent): it is moved aside as `<name>.bad-<seconds>`, and the day and month it was
//! found in count as unknown (`Ledger::restore`).

use base_fs::atomic::AtomicWrite;
use base_fs::{ReadCap, ReadError, move_aside, read_text_capped};
use base_time::UnixSeconds;
use inference_settings::{Found, Ledger, Persist};
use std::path::{Path, PathBuf};

/// A ledger kept in `file`: what it holds is read now and every record is written back. A file
/// that is missing starts it empty; one that cannot be read is set aside and its period stops.
#[must_use]
pub fn open_at(file: PathBuf, now: UnixSeconds) -> Ledger {
    let found = match read_text_capped(&file, ReadCap::mib(8)) {
        Ok(text) => Found::Text(text),
        Err(ReadError::NotFound { .. }) => Found::Missing,
        Err(error) => Found::Unreadable(chain(&error)),
    };
    let (ledger, lost) = Ledger::restore(found, now);
    if let Some(why) = &lost {
        eprintln!("inferd: spend: {}: {why}", file.display());
        set_aside(&file, now);
    }
    let ledger = ledger.persisting(Box::new(Book { file }));
    if lost.is_some() {
        // Keep the loss at once: a restart before the first record must not forget it.
        ledger.save_now();
    }
    ledger
}

/// The ledger's document on disk.
#[derive(Debug)]
struct Book {
    file: PathBuf,
}

impl Persist for Book {
    fn save(&self, document: Result<&str, &dyn std::error::Error>) {
        // On disk before it replaces the old file, or a power cut leaves an empty ledger.
        let written = match document {
            Ok(text) => AtomicWrite::PRIVATE.write(&self.file, text.as_bytes()),
            Err(error) => Err(std::io::Error::other(chain(error))),
        };
        if let Err(why) = written {
            eprintln!("inferd: spend: {}: {why}", self.file.display());
        }
    }
}

/// `error` and its causes, one after the other.
fn chain(error: &dyn std::error::Error) -> String {
    let mut text = error.to_string();
    let mut next = error.source();
    while let Some(cause) = next {
        text.push_str(": ");
        text.push_str(&cause.to_string());
        next = cause.source();
    }
    text
}

/// Moves a ledger file that could not be read out of the way (`base_fs::move_aside`), so it can be
/// looked at and the next save does not write over it.
fn set_aside(file: &Path, now: UnixSeconds) {
    match move_aside(file, now) {
        Ok(aside) => eprintln!(
            "inferd: spend: {} could not be read; moved to {}. Spending limits stop requests \
             until the day and the month end.",
            file.display(),
            aside.display()
        ),
        Err(why) => eprintln!("inferd: spend: {why}"),
    }
}

#[cfg(test)]
mod tests;
