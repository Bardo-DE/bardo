//! How much a surface moves: the level the token table is written against ([`MotionLevel`]).
//!
//! Motion is an accessibility preference, not a style (design/30-CATALOGUE.md section 1.1):
//! the person's choice is [`Motion`](appearance_settings::Motion) (`appearance.motion_level`),
//! [`MotionLevel`] is what a root is drawn at, and `Reduced` is macOS's "Reduce motion" with
//! cross-fade transitions on.

use base_words::Word;
use serde::{Deserialize, Serialize};

/// The motion level a `.ds` root is drawn at: a [`Motion`](appearance_settings::Motion) with the desktop's answer applied.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Copy, Hash, Default, Word)]
#[serde(rename_all = "snake_case")]
pub enum MotionLevel {
    /// The tables of design/30 section 1.2 as written.
    #[default]
    Standard,
    /// Every moving duration is `--t-quick`, springs are critically damped, drags track 1:1.
    Reduced,
}
