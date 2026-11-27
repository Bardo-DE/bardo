//! How much a surface moves, as a person picks it.
//!
//! Motion is an accessibility preference, not a style (design/30-CATALOGUE.md section 1.1):
//! [`Motion`] is `appearance.motion_level`, and `Reduced` is macOS's "Reduce motion" with
//! cross-fade transitions on.

use base_words::Word;
use serde::{Deserialize, Serialize};

/// How much the window moves, as a person chooses it.
///
/// The desktop's own reduced-motion preference always wins over `Standard` when the look is
/// resolved, so a person who asked the desktop to reduce motion is never moved.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Copy, Hash, Default, Word)]
#[serde(rename_all = "snake_case")]
pub enum Motion {
    /// The macOS defaults, unless the desktop asks for reduced motion.
    #[default]
    Standard,
    /// Slides, scales and springs become a cross-fade; nothing overshoots.
    Reduced,
}
