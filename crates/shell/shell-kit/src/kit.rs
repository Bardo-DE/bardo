//! The shell's kit: its metric tokens and paints. Its component sheets are placed in the components section, each after the sheet it follows.

use crate::sheets::SHEETS;
use crate::tokens::control_center::{ControlCenterSize, ControlCenterTuned};
use crate::tokens::dock::DockToken;
use crate::tokens::launcher::LauncherToken;
use crate::tokens::notifications::NotificationToken;
use crate::tokens::osd::OsdToken;
use crate::tokens::widgets::WidgetGrid;
use std::sync::LazyLock;
use ui_components::style::css::document::placed;
use ui_components::style::kit::{Kit, KitRank, Kits, Vocabulary};
use ui_components::style::tokens::set::{Place, TokenSet};

/// The shell's contribution to the stylesheet and the linter.
pub static KIT: Kit = Kit {
    rank: KitRank::Shell,
    tokens: &[
        TokenSet::of::<ControlCenterSize>().at(Place::Ladder),
        TokenSet::of::<ControlCenterTuned>().at(Place::Metrics),
        TokenSet::of::<DockToken>().at(Place::Metrics),
        TokenSet::of::<LauncherToken>().at(Place::Metrics),
        TokenSet::of::<OsdToken>().at(Place::Metrics),
        TokenSet::of::<NotificationToken>().at(Place::Metrics),
        TokenSet::of::<WidgetGrid>().at(Place::Metrics),
    ],
    sections: &[],
    sheets: &SHEETS,
    vocabulary: Vocabulary::NONE,
};

/// Every kit the shell is drawn with: the design system's (style, motion, the components), the
/// shell's, and the app-facing parts' (`ui_shell`). The shell's come first among the shell-rank
/// kits, so the cascade is the one the stylesheet had when it was one kit.
#[must_use]
pub fn kits() -> Kits {
    Kits::of(&[
        &ui_components::style::kit::STYLE_KIT,
        &ui_components::motion::kit::KIT,
        &ui_components::KIT,
        &KIT,
        &ui_shell::KIT,
    ])
}

/// Every rule a shell surface draws with, in cascade order. Built once, on first use.
pub fn stylesheet() -> &'static str {
    static SHEET: LazyLock<String> = LazyLock::new(|| kits().stylesheet());
    SHEET.as_str()
}

/// Every component sheet, the design system's and the shell's, `(name, css)`, in the order the
/// stylesheet appends them.
#[must_use]
pub fn component_sheets() -> Vec<(&'static str, &'static str)> {
    let mut extra = SHEETS.to_vec();
    extra.extend(ui_shell::KIT.sheets.iter().copied());
    placed(&ui_components::component_sheets(), &extra)
}
