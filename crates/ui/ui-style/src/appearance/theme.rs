//! Which palette a surface paints: the two-way answer the stylesheet needs ([`Scheme`]), which a
//! person's three-way choice ([`Theme`](appearance_settings::Theme)) resolves to.
//!
//! Bardo resolves the scheme in Rust and always writes an explicit `data-theme`, never leaving
//! `System` to a `prefers-color-scheme` media guard (design/05-MOTION.md section 9 rule 11,
//! [`crate::resolve`]).

use base_words::Word;
use serde::{Deserialize, Serialize};

/// The palette a surface actually paints: a [`Theme`](appearance_settings::Theme) with "follow the desktop" answered.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Copy, Hash, Default, Word)]
#[serde(rename_all = "snake_case")]
pub enum Scheme {
    /// The light Post palette.
    #[default]
    Light,
    /// The dark Post palette.
    Dark,
}
