//! How colourful our app icons are drawn (`icons.style`, design/08-ICONS.md section 2.10).

use base_words::Word;
use serde::{Deserialize, Serialize};

/// Which dialect our app icons are drawn in (`icons.style`, design/08-ICONS.md section 2.10).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize, Word)]
#[serde(rename_all = "snake_case")]
#[word(case = snake)]
pub enum IconStyle {
    /// Each app as designed; third-party icons as they come.
    #[default]
    Colour,
    /// Chroma held down (the shipped cap 0.07 scaled to 0.04).
    Muted,
    /// One hue, the tint's, tone on tone.
    Monochrome,
}
