//! The recently-viewed history the viewer writes and the launcher reads: its types.

use crate::label::ResumeLabel;
use anyview_core::{FilePath, FormatKind};

/// One viewed file, as the launcher needs it: which file, what kind, when, and where it was left.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct HistoryEntry {
    /// The file.
    pub path: FilePath,
    /// What it was when viewed.
    pub kind: FormatKind,
    /// When it was last viewed.
    pub viewed: u64,
    /// Where it was left, for a row's subtitle.
    pub label: ResumeLabel,
}

/// The whole of `history.json`: files by recency, the most recently viewed first, each path once.
#[derive(Debug, Clone, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
pub struct History {
    /// Newest first.
    #[serde(default)]
    pub entries: Vec<HistoryEntry>,
}
