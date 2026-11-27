//! Which palette a surface resolves to, as a person picks it.

use base_words::Word;
use serde::{Deserialize, Serialize};

/// Which palette the window resolves to.
///
/// Three states and not a bool: "follow the desktop" is a different choice from "light",
/// and a client that cannot express it either ignores the desktop or cannot be overridden
/// when the desktop is wrong.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Copy, Hash, Default, Word)]
#[serde(rename_all = "snake_case")]
pub enum Theme {
    /// Follow the desktop: "Auto", as macOS names it (the stored word stays `system`).
    #[default]
    #[word(label = "Auto")]
    System,
    /// The light palette, even when the desktop is dark.
    Light,
    /// The dark palette, even when the desktop is light.
    Dark,
}
