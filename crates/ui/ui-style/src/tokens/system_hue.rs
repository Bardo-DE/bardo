//! Apple's thirteen system colours (Human Interface Guidelines, Color, 2025): each hue in the
//! four forms the platform gives it, light and dark, at ordinary and at increased contrast.
//!
//! The accents, the signal colours and the tag colours all read their hues here, so a hue is
//! written once. The values are Apple's own (`color.json` of the HIG, 2025).
//!
//! As a token a hue is a tag colour (`--tag-red` … `--tag-gray`): the thirteen colours a person can
//! give a tag, resolved like the accents. A tag's colour is a choice the person made, so it stays
//! one of these thirteen and never a value of the app's own; an app stores the hue and paints
//! `var(--tag-<hue>)`.

use crate::appearance::{system::Contrast, theme::Scheme};
use crate::tokens::hex::Hex;
use crate::tokens::token::{CssValue, Token, TokenScope};
use ui_core::word::Word;

/// One of the thirteen system hues, in the order the HIG lists them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Word, Token)]
#[token(prefix = "tag-", kind = fixed, css = tag_css)]
pub enum SystemHue {
    /// systemRed.
    Red,
    /// systemOrange.
    Orange,
    /// systemYellow.
    Yellow,
    /// systemGreen.
    Green,
    /// systemMint.
    Mint,
    /// systemTeal.
    Teal,
    /// systemCyan.
    Cyan,
    /// systemBlue.
    Blue,
    /// systemIndigo.
    Indigo,
    /// systemPurple.
    Purple,
    /// systemPink.
    Pink,
    /// systemBrown.
    Brown,
    /// systemGray (Finder's Gray tag).
    Gray,
}

/// A hue's four values: ordinary and increased contrast, in each scheme.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct HueShades {
    /// Light, ordinary contrast.
    pub light: Hex,
    /// Light, increased contrast.
    pub light_increased: Hex,
    /// Dark, ordinary contrast.
    pub dark: Hex,
    /// Dark, increased contrast.
    pub dark_increased: Hex,
}

impl HueShades {
    /// The shade for `scheme` at `contrast`.
    #[must_use]
    pub const fn pick(self, scheme: Scheme, contrast: Contrast) -> Hex {
        match (scheme, contrast) {
            (Scheme::Light, Contrast::Normal) => self.light,
            (Scheme::Light, Contrast::High) => self.light_increased,
            (Scheme::Dark, Contrast::Normal) => self.dark,
            (Scheme::Dark, Contrast::High) => self.dark_increased,
        }
    }
}

const fn hex(value: u32) -> Hex {
    let [_, red, green, blue] = value.to_be_bytes();
    Hex([red, green, blue])
}

const fn shades(light: u32, light_increased: u32, dark: u32, dark_increased: u32) -> HueShades {
    HueShades {
        light: hex(light),
        light_increased: hex(light_increased),
        dark: hex(dark),
        dark_increased: hex(dark_increased),
    }
}

impl SystemHue {
    /// The hue's four values.
    #[must_use]
    pub const fn shades(self) -> HueShades {
        match self {
            SystemHue::Red => shades(0x00FF_383C, 0x00E9_152D, 0x00FF_4245, 0x00FF_6165),
            SystemHue::Orange => shades(0x00FF_8D28, 0x00C5_5300, 0x00FF_9230, 0x00FF_A056),
            SystemHue::Yellow => shades(0x00FF_CC00, 0x00A1_6A00, 0x00FF_D600, 0x00FE_DF43),
            SystemHue::Green => shades(0x0034_C759, 0x0000_8932, 0x0030_D158, 0x004A_D968),
            SystemHue::Mint => shades(0x0000_C8B3, 0x0000_8575, 0x0000_DAC3, 0x0054_DFCB),
            SystemHue::Teal => shades(0x0000_C3D0, 0x0000_8198, 0x0000_D2E0, 0x003B_DDEC),
            SystemHue::Cyan => shades(0x0000_C0E8, 0x0000_7EAE, 0x003C_D3FE, 0x006D_D9FF),
            SystemHue::Blue => shades(0x0000_88FF, 0x001E_6EF4, 0x0000_91FF, 0x005C_B8FF),
            SystemHue::Indigo => shades(0x0061_55F5, 0x0056_4ADE, 0x006D_7CFF, 0x00A7_AAFF),
            SystemHue::Purple => shades(0x00CB_30E0, 0x00B0_2FC2, 0x00DB_34F2, 0x00EA_8DFF),
            SystemHue::Pink => shades(0x00FF_2D55, 0x00E7_124D, 0x00FF_375F, 0x00FF_8AC4),
            SystemHue::Brown => shades(0x00AC_7F5E, 0x0095_6D51, 0x00B7_8A66, 0x00DB_A679),
            SystemHue::Gray => shades(0x008E_8E93, 0x006C_6C70, 0x0098_989D, 0x00AE_AEB2),
        }
    }

    /// The hue in `scheme` at `contrast`.
    #[must_use]
    pub const fn value(self, scheme: Scheme, contrast: Contrast) -> Hex {
        self.shades().pick(scheme, contrast)
    }
}

/// A hue as the stylesheet writes its tag colour: its value in the scope's scheme.
fn tag_css(token: SystemHue, scope: TokenScope) -> CssValue {
    CssValue::computed(token.value(scope.scheme, Contrast::Normal).css())
}

#[cfg(test)]
mod tests {
    use super::SystemHue;
    use crate::appearance::{system::Contrast, theme::Scheme};
    use ui_core::word::Word;

    #[test]
    fn the_thirteen_come_in_the_order_the_guidelines_list_them() {
        let slugs: Vec<&str> = SystemHue::ALL.iter().map(|hue| hue.slug()).collect();
        assert_eq!(
            slugs,
            [
                "red", "orange", "yellow", "green", "mint", "teal", "cyan", "blue", "indigo",
                "purple", "pink", "brown", "gray"
            ]
        );
    }

    #[test]
    fn each_hue_is_the_tag_colour_of_the_same_name() {
        for hue in SystemHue::ALL.iter().copied() {
            assert_eq!(hue.var().as_str(), format!("--tag-{}", hue.slug()));
        }
    }

    #[test]
    fn blue_is_apples_blue_in_all_four_forms() {
        let got = [
            (Scheme::Light, Contrast::Normal),
            (Scheme::Light, Contrast::High),
            (Scheme::Dark, Contrast::Normal),
            (Scheme::Dark, Contrast::High),
        ]
        .map(|(scheme, contrast)| SystemHue::Blue.value(scheme, contrast).css());
        assert_eq!(
            got,
            ["#0088ff", "#1e6ef4", "#0091ff", "#5cb8ff"].map(str::to_owned)
        );
    }
}
