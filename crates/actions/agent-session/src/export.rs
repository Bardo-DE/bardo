//! `export`: one session as a stable JSON document. Entries only: handle labels and no values,
//! no secrets, in log order, with no positions (the order is the position). An ACP-shaped
//! transcript is a rendering of this, not another form.

use crate::codec::{Logged, Read};
use crate::entry::{Seq, SessionEntry};
use provenance::SessionId;
use serde::{Deserialize, Serialize};

/// The version of the export document.
pub const EXPORT_VERSION: u32 = 1;

/// A session, exported.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionExport {
    /// The document's version.
    pub export_version: u32,
    /// The session.
    pub session: SessionId,
    /// Its entries, in order (a row that cannot be read fails the export).
    pub entries: Vec<SessionEntry>,
}

/// Why an export or an import failed.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ExportFault {
    /// A row could not be read; an export that dropped it would say less than the log.
    #[error("the entry at {0:?} cannot be read")]
    Unreadable(Seq),
    /// The document is not valid JSON of this form.
    #[error("the document is not a session export")]
    Malformed,
    /// A document version this crate does not know.
    #[error("unknown export version {0}")]
    UnknownVersion(u32),
}

/// The export of `session`'s rows.
///
/// # Errors
///
/// Returns `ExportFault::Unreadable` when a row cannot be read: an export that dropped it would say
/// less than the log.
pub fn export(session: &SessionId, rows: &[Logged]) -> Result<SessionExport, ExportFault> {
    let mut entries = Vec::new();
    for row in rows {
        match &row.read {
            Read::Entry(e) => entries.push(e.as_ref().clone()),
            Read::Unreadable(_) => return Err(ExportFault::Unreadable(row.seq)),
        }
    }
    Ok(SessionExport {
        export_version: EXPORT_VERSION,
        session: session.clone(),
        entries,
    })
}

/// The document as JSON text: field order is the struct's, set order is `BTreeSet`'s, so the
/// same export is the same bytes.
///
/// # Errors
///
/// Returns `ExportFault` when the document cannot be written as JSON.
pub fn to_json(export: &SessionExport) -> Result<String, ExportFault> {
    serde_json::to_string_pretty(export)
        .ok()
        .ok_or(ExportFault::Malformed)
}

/// The export in `json`; the version is read before the rest, so a newer document is
/// `UnknownVersion`, not `Malformed`.
///
/// # Errors
///
/// Returns `ExportFault::UnknownVersion` for a document version this crate does not know, and
/// `ExportFault::Malformed` when `json` is not a valid document of this form.
pub fn from_json(json: &str) -> Result<SessionExport, ExportFault> {
    let doc: serde_json::Value = serde_json::from_str(json)
        .ok()
        .ok_or(ExportFault::Malformed)?;
    let version = doc
        .get("export_version")
        .and_then(serde_json::Value::as_u64)
        .and_then(|v| u32::try_from(v).ok())
        .ok_or(ExportFault::Malformed)?;
    if version != EXPORT_VERSION {
        return Err(ExportFault::UnknownVersion(version));
    }
    serde_json::from_value(doc)
        .ok()
        .ok_or(ExportFault::Malformed)
}
