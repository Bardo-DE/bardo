//! Every `Word` enum that is stored in a settings file writes the same word as its attribute:
//! a serde rename and a slug that drifted apart would make a saved choice unreadable.

use crate::appearance::{material::Material, motion::MotionLevel, peek::PeekMode, theme::Scheme};
use crate::tokens::label_hue::LabelHue;
use appearance_settings::{Accent, Motion, Theme, Transparency};
use ui_core::testing::word_matches_serde;

#[test]
fn stored_vocabularies_serialise_as_their_slugs() {
    word_matches_serde::<Accent>();
    word_matches_serde::<LabelHue>();
    word_matches_serde::<Material>();
    word_matches_serde::<Motion>();
    word_matches_serde::<MotionLevel>();
    word_matches_serde::<PeekMode>();
    word_matches_serde::<Scheme>();
    word_matches_serde::<Theme>();
    word_matches_serde::<Transparency>();
}
