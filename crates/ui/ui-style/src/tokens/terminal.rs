//! The terminal's palette (a terminal app draws its own cells, so it reads these as data): the
//! sixteen ANSI colours, the default foreground and background, the cursor and the selection,
//! for each scheme.
//!
//! Foreground and background are the card's ink and surface in the scheme, so a terminal sits
//! in a Space's card like any other content (design/21-SPACES.md: the card follows the
//! scheme, the Space paints the frame around it). The cursor and the selection follow the
//! accent, as a text caret and a selected row do. The hues are Apple's system colours at
//! increased contrast (`system_hue`): on the dark ground those are Apple's own; on white the
//! normals are the increased-contrast values and each bright is one step (0.07 of OKLCH
//! lightness) darker, in the same hue. Every ANSI colour reads on the background at
//! 4.5:1 or better, so the "black" of a dark scheme and the "white" of a light one are lifted
//! or deepened from the usual: `ansi_colours_read_on_the_background` gates all sixteen in both.

use crate::appearance::theme::Scheme;
use crate::tokens::accent_table::accent_of;
use crate::tokens::hex::{Colour, Hex};
use crate::tokens::token::{CssValue, Token, TokenScope};
use appearance_settings::Accent;
use base_words::Word;

const fn rgb(value: u32) -> Colour {
    let [_, red, green, blue] = value.to_be_bytes();
    Colour::Solid(Hex([red, green, blue]))
}

/// One colour of the terminal palette.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Word, Token)]
#[token(prefix = "term-", kind = fixed, css = terminal_css)]
pub enum TerminalColour {
    /// `--term-fg`: the default foreground.
    Fg,
    /// `--term-bg`: the default background.
    Bg,
    /// `--term-cursor`: the cursor block, the accent's text colour.
    Cursor,
    /// `--term-selection`: the wash behind selected cells, the accent's own.
    Selection,
    /// `--term-ansi-0`: black.
    #[token(name = "ansi-0")]
    Black,
    /// `--term-ansi-1`: red.
    #[token(name = "ansi-1")]
    Red,
    /// `--term-ansi-2`: green.
    #[token(name = "ansi-2")]
    Green,
    /// `--term-ansi-3`: yellow.
    #[token(name = "ansi-3")]
    Yellow,
    /// `--term-ansi-4`: blue.
    #[token(name = "ansi-4")]
    Blue,
    /// `--term-ansi-5`: magenta.
    #[token(name = "ansi-5")]
    Magenta,
    /// `--term-ansi-6`: cyan.
    #[token(name = "ansi-6")]
    Cyan,
    /// `--term-ansi-7`: white.
    #[token(name = "ansi-7")]
    White,
    /// `--term-ansi-8`: bright black.
    #[token(name = "ansi-8")]
    BrightBlack,
    /// `--term-ansi-9`: bright red.
    #[token(name = "ansi-9")]
    BrightRed,
    /// `--term-ansi-10`: bright green.
    #[token(name = "ansi-10")]
    BrightGreen,
    /// `--term-ansi-11`: bright yellow.
    #[token(name = "ansi-11")]
    BrightYellow,
    /// `--term-ansi-12`: bright blue.
    #[token(name = "ansi-12")]
    BrightBlue,
    /// `--term-ansi-13`: bright magenta.
    #[token(name = "ansi-13")]
    BrightMagenta,
    /// `--term-ansi-14`: bright cyan.
    #[token(name = "ansi-14")]
    BrightCyan,
    /// `--term-ansi-15`: bright white.
    #[token(name = "ansi-15")]
    BrightWhite,
}

/// A measure of the terminal's cell grid that is the same in every scheme.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Word, Token)]
#[token(prefix = "term-", kind = fixed)]
pub enum TerminalMetric {
    /// `--term-font`: the system's own monospace family, for a terminal's cells. Space Mono
    /// (`--font-code`) has no box drawing, CJK or emoji; the system's monospace covers them and
    /// falls back per glyph. No shipped face leads this stack, on purpose.
    #[token(name = "font", value = "monospace")]
    Font,
    /// `--term-lh`: a row's height as a multiple of the font size (`--term-font`); rows this close
    /// let box drawing join from one to the next.
    #[token(name = "lh", value = "1.2")]
    LineHeight,
}

impl TerminalColour {
    /// The sixteen ANSI colours, in index order: 0 to 7, then their bright forms 8 to 15.
    pub const ANSI: [TerminalColour; 16] = [
        TerminalColour::Black,
        TerminalColour::Red,
        TerminalColour::Green,
        TerminalColour::Yellow,
        TerminalColour::Blue,
        TerminalColour::Magenta,
        TerminalColour::Cyan,
        TerminalColour::White,
        TerminalColour::BrightBlack,
        TerminalColour::BrightRed,
        TerminalColour::BrightGreen,
        TerminalColour::BrightYellow,
        TerminalColour::BrightBlue,
        TerminalColour::BrightMagenta,
        TerminalColour::BrightCyan,
        TerminalColour::BrightWhite,
    ];

    /// The colour in `scheme` with `accent` as the accent the cursor and selection follow.
    #[must_use]
    pub fn value(self, scheme: Scheme, accent: Accent) -> Colour {
        let roles = accent_of(accent, scheme);
        let (light, dark) = match self {
            TerminalColour::Cursor => return Colour::Solid(roles.text),
            TerminalColour::Selection => return roles.wash_colour(),
            other => other.fixed(),
        };
        match scheme {
            Scheme::Light => light,
            Scheme::Dark => dark,
        }
    }

    /// The colours that do not follow the accent, `(light, dark)`.
    fn fixed(self) -> (Colour, Colour) {
        match self {
            // The card's ink and surface (`ColourToken::Ink`, `Surface`), the dark ground being
            // the window paper's, which a terminal sits deepest in.
            TerminalColour::Fg => (rgb(0x0020_2020), rgb(0x00E8_E8E8)),
            TerminalColour::Bg => (rgb(0x00FF_FFFF), rgb(0x001E_1E1E)),
            TerminalColour::Black => (rgb(0x0030_3030), rgb(0x008E_8E93)),
            TerminalColour::Red => (rgb(0x00E9_152D), rgb(0x00FF_6165)),
            TerminalColour::Green => (rgb(0x0000_8932), rgb(0x004A_D968)),
            TerminalColour::Yellow => (rgb(0x00A1_6A00), rgb(0x00FE_DF43)),
            TerminalColour::Blue => (rgb(0x001E_6EF4), rgb(0x005C_B8FF)),
            TerminalColour::Magenta => (rgb(0x00B0_2FC2), rgb(0x00EA_8DFF)),
            TerminalColour::Cyan => (rgb(0x0000_8198), rgb(0x003B_DDEC)),
            TerminalColour::White => (rgb(0x005C_5C5C), rgb(0x00D1_D1D6)),
            TerminalColour::BrightBlack => (rgb(0x006B_6B6B), rgb(0x00AE_AEB2)),
            TerminalColour::BrightRed => (rgb(0x00C7_0322), rgb(0x00FF_9093)),
            TerminalColour::BrightGreen => (rgb(0x0000_7128), rgb(0x0080_E495)),
            TerminalColour::BrightYellow => (rgb(0x0086_5803), rgb(0x00FE_E97B)),
            TerminalColour::BrightBlue => (rgb(0x0000_57DA), rgb(0x008D_CDFF)),
            TerminalColour::BrightMagenta => (rgb(0x0099_08AB), rgb(0x00F0_AFFF)),
            TerminalColour::BrightCyan => (rgb(0x0006_6B7E), rgb(0x0076_E7F2)),
            TerminalColour::BrightWhite => (rgb(0x004A_4A4A), rgb(0x00F5_F5F7)),
            TerminalColour::Cursor | TerminalColour::Selection => {
                (rgb(0x0000_0000), rgb(0x0000_0000))
            }
        }
    }
}

/// A terminal token as the stylesheet writes it: the cursor and the selection name the accent's
/// own variables, so they follow whichever accent a root selects; the rest is the table's value.
fn terminal_css(token: TerminalColour, scope: TokenScope) -> CssValue {
    match token {
        TerminalColour::Cursor => CssValue::fixed("var(--accent-text)"),
        TerminalColour::Selection => CssValue::fixed("var(--accent-soft)"),
        other => CssValue::computed(other.value(scope.scheme, Accent::Blue).css()),
    }
}

#[cfg(test)]
mod tests {
    use super::TerminalColour;
    use crate::appearance::theme::Scheme;
    use crate::tokens::hex::Hex;
    use crate::tokens::token::{Token, TokenScope};
    use appearance_settings::Accent;
    use base_words::Word;
    use ui_core::colour::contrast::ratio;

    fn css(token: TerminalColour, scheme: Scheme) -> String {
        token.value(scheme, Accent::Blue).css()
    }

    #[test]
    fn the_foreground_reads_on_the_background() {
        for scheme in Scheme::ALL.iter().copied() {
            let (fg, bg) = (
                css(TerminalColour::Fg, scheme),
                css(TerminalColour::Bg, scheme),
            );
            let got = ratio(&fg, &bg).expect("hex pair");
            assert!(got >= 4.5, "{scheme:?}: fg {fg} on bg {bg} is {got:.2}");
        }
    }

    #[test]
    fn ansi_colours_read_on_the_background() {
        let mut failures = Vec::new();
        for scheme in Scheme::ALL.iter().copied() {
            let bg = css(TerminalColour::Bg, scheme);
            for (index, token) in TerminalColour::ANSI.iter().copied().enumerate() {
                let colour = css(token, scheme);
                let got = ratio(&colour, &bg).expect("hex pair");
                if got < 4.5 {
                    failures.push(format!(
                        "{scheme:?} ansi {index} {colour} on {bg}: {got:.2}"
                    ));
                }
            }
        }
        assert!(failures.is_empty(), "{failures:#?}");
    }

    #[test]
    fn the_cursor_and_the_foreground_over_the_selection_read_for_every_accent() {
        let mut failures = Vec::new();
        for scheme in Scheme::ALL.iter().copied() {
            let bg = Hex::parse(&css(TerminalColour::Bg, scheme)).expect("hex");
            for accent in Accent::ALL.iter().copied() {
                let cursor = TerminalColour::Cursor.value(scheme, accent).css();
                let wash = match TerminalColour::Selection.value(scheme, accent) {
                    crate::tokens::hex::Colour::Alpha(fill, alpha) => fill.over(alpha, bg).css(),
                    crate::tokens::hex::Colour::Solid(fill) => fill.css(),
                };
                let fg = css(TerminalColour::Fg, scheme);
                for (what, fore, back, floor) in [
                    ("cursor on bg", cursor, bg.css(), 3.0),
                    ("fg on selection", fg, wash, 4.5),
                ] {
                    let got = ratio(&fore, &back).expect("hex pair");
                    if got < floor {
                        failures.push(format!("{scheme:?} {accent:?} {what}: {got:.2}"));
                    }
                }
            }
        }
        assert!(failures.is_empty(), "{failures:#?}");
    }

    #[test]
    fn the_cursor_and_selection_name_the_accent_and_the_rest_are_hex() {
        let scope = TokenScope::BASE;
        assert_eq!(
            TerminalColour::Cursor.css_value(scope).as_str(),
            "var(--accent-text)"
        );
        assert_eq!(
            TerminalColour::Selection.css_value(scope).as_str(),
            "var(--accent-soft)"
        );
        assert_eq!(TerminalColour::Black.var().as_str(), "--term-ansi-0");
        assert_eq!(TerminalColour::BrightWhite.var().as_str(), "--term-ansi-15");
        for token in TerminalColour::ALL.iter().copied() {
            let dark = token.css_value(scope.in_scheme(Scheme::Dark));
            assert!(
                dark.as_str().starts_with('#') || dark.as_str().starts_with("var("),
                "{token:?}: {}",
                dark.as_str()
            );
        }
    }
}
