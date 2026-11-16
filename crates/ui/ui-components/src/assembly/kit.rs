//! The components' kit (the utilities, then every component sheet in its one registered order),
//! and the kits a surface is drawn with.

use crate::assembly::selectors;
use crate::assembly::sheets::SHEETS;
use std::borrow::Cow;
use ui_style::css::document::{placed, sheets, utilities_css};
use ui_style::kit::{Kit, KitRank, Kits, STYLE_KIT, Section, Vocabulary};

/// The components' contribution to the stylesheet.
pub static KIT: Kit = Kit {
    rank: KitRank::Components,
    tokens: &[],
    sections: &[
        Section {
            name: "utilities",
            css: utilities,
        },
        Section {
            name: "components",
            css: components,
        },
    ],
    sheets: &[],
    vocabulary: Vocabulary {
        public_classes: selectors::classes,
        public_attributes: selectors::attributes,
        ..Vocabulary::NONE
    },
};

/// Every kit the design system ships: style, motion and the components. A crate that adds its own
/// (`ui_shell::kits()`) lists these and its own.
#[must_use]
pub fn kits() -> Kits {
    Kits::of(&[&STYLE_KIT, &ui_motion::kit::KIT, &KIT])
}

fn utilities(_: &Kits) -> Cow<'static, str> {
    Cow::Owned(utilities_css())
}

fn components(kits: &Kits) -> Cow<'static, str> {
    Cow::Owned(sheets(&placed(&SHEETS, &kits.sheets())))
}
