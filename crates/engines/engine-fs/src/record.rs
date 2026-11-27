//! Reading and writing the record an install leaves beside its tree.

use base_fs::atomic::AtomicWrite;
use engine_manifest::{Record, RecordError};
use std::path::Path;

/// The record at `path`.
///
/// # Errors
/// See [`RecordError`].
pub fn read_record(path: &Path) -> Result<Record, RecordError> {
    let text = std::fs::read(path).map_err(|e| RecordError::Io(e.kind()))?;
    serde_json::from_slice(&text).map_err(|e| RecordError::Malformed(e.to_string()))
}

/// Writes `record` at `path`, whole (`base_fs`'s atomic write).
///
/// # Errors
/// See [`RecordError`].
pub fn write_record(record: &Record, path: &Path) -> Result<(), RecordError> {
    let text =
        serde_json::to_vec_pretty(record).map_err(|e| RecordError::Malformed(e.to_string()))?;
    AtomicWrite::SHARED
        .write(path, &text)
        .map_err(|e| RecordError::Io(e.kind()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    #[test]
    fn a_record_round_trips_and_a_missing_or_foreign_file_is_an_error() {
        let dir = tempfile::tempdir().expect("scratch dir");
        let path = dir.path().join("installed.json");
        let record = Record {
            engine: "llama_cpp".into(),
            version: "b1".into(),
            backend: "cpu".into(),
            seal: "00".into(),
            entries: BTreeMap::from([("a".to_owned(), "11".to_owned())]),
            venv_seal: None,
        };
        write_record(&record, &path).expect("write");
        assert_eq!(read_record(&path), Ok(record));
        assert_eq!(
            read_record(&dir.path().join("gone")),
            Err(RecordError::Io(std::io::ErrorKind::NotFound))
        );
        std::fs::write(&path, "[]").expect("write");
        assert!(matches!(read_record(&path), Err(RecordError::Malformed(_))));
    }
}
