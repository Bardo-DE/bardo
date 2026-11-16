//! What the on-screen indicator can show.

use serde::{Deserialize, Serialize};
use ui_core::word::Word;

/// What the indicator shows.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Word)]
#[serde(rename_all = "snake_case")]
pub enum OsdKind {
    Volume,
    Brightness,
}
