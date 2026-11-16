//! The Candy shelf as label hues: `--c-{red,amber,green,blue,violet}{,-deep,-soft}`
//! (design/03-COLOR.md section 15, design/07-LOOKS.md section 6.2).
//!
//! Values are `C:24-28` and `C:155-170` verbatim; that they name labels at all is proposed
//! (design/07-LOOKS.md section 11).

use super::hex::Hex;
use super::token::{CssValue, Token, TokenScope};
use crate::appearance::theme::Scheme;
use serde::{Deserialize, Serialize};
use ui_core::word::Word;

/// One label hue.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Word)]
#[serde(rename_all = "snake_case")]
pub enum LabelHue {
    /// Red.
    Red,
    /// Amber.
    Amber,
    /// Green.
    Green,
    /// Blue.
    Blue,
    /// Violet.
    Violet,
}

/// Which member of a hue's family: the base, the text-safe deep, or the soft tint.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Word)]
pub enum HueMember {
    /// `--c-<hue>`.
    Base,
    /// `--c-<hue>-deep`: text-safe on the soft tint.
    Deep,
    /// `--c-<hue>-soft`: the chip ground.
    Soft,
}

/// One label hue's member as a token: `--c-red`, `--c-red-deep`, `--c-red-soft`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Word, Token)]
#[token(prefix = "c-", kind = fixed, css = hue_css)]
pub enum HueColour {
    /// `--c-red`.
    Red,
    /// `--c-red-deep`.
    RedDeep,
    /// `--c-red-soft`.
    RedSoft,
    /// `--c-amber`.
    Amber,
    /// `--c-amber-deep`.
    AmberDeep,
    /// `--c-amber-soft`.
    AmberSoft,
    /// `--c-green`.
    Green,
    /// `--c-green-deep`.
    GreenDeep,
    /// `--c-green-soft`.
    GreenSoft,
    /// `--c-blue`.
    Blue,
    /// `--c-blue-deep`.
    BlueDeep,
    /// `--c-blue-soft`.
    BlueSoft,
    /// `--c-violet`.
    Violet,
    /// `--c-violet-deep`.
    VioletDeep,
    /// `--c-violet-soft`.
    VioletSoft,
}

impl HueColour {
    /// The hue and member this token is: the family in its order, each hue's three members.
    #[must_use]
    pub fn parts(self) -> (LabelHue, HueMember) {
        let index = Self::ALL
            .iter()
            .position(|token| *token == self)
            .unwrap_or(0);
        let members = HueMember::ALL.len();
        (
            LabelHue::ALL[index / members],
            HueMember::ALL[index % members],
        )
    }
}

impl LabelHue {
    /// The token for one member: `--c-red-deep`.
    #[must_use]
    pub fn colour(self, member: HueMember) -> HueColour {
        let hue = LabelHue::ALL
            .iter()
            .position(|hue| *hue == self)
            .unwrap_or(0);
        let member = HueMember::ALL
            .iter()
            .position(|m| *m == member)
            .unwrap_or(0);
        HueColour::ALL[hue * HueMember::ALL.len() + member]
    }
}

/// A hue member as the stylesheet writes it, in the scope's scheme.
fn hue_css(token: HueColour, scope: TokenScope) -> CssValue {
    let (hue, member) = token.parts();
    CssValue::computed(hue.value(member, scope.scheme).css())
}

impl LabelHue {
    /// The member's value in `scheme`.
    #[must_use]
    pub fn value(self, member: HueMember, scheme: Scheme) -> Hex {
        let [base, deep, soft] = self.family(scheme);
        let rgb = match member {
            HueMember::Base => base,
            HueMember::Deep => deep,
            HueMember::Soft => soft,
        };
        let [_, red, green, blue] = rgb.to_be_bytes();
        Hex([red, green, blue])
    }

    /// Base, deep and soft as `0xRRGGBB` (design/03-COLOR.md section 15).
    fn family(self, scheme: Scheme) -> [u32; 3] {
        match (self, scheme) {
            (LabelHue::Red, Scheme::Light) => [0x00E8_483C, 0x00B7_352B, 0x00FB_E3E1],
            (LabelHue::Amber, Scheme::Light) => [0x00F0_A81E, 0x008E_5A05, 0x00FA_EBD2],
            (LabelHue::Green, Scheme::Light) => [0x0028_B24A, 0x001A_7A33, 0x00DC_F1E1],
            (LabelHue::Blue, Scheme::Light) => [0x002B_7CFF, 0x000B_5FE0, 0x00E2_EBFF],
            (LabelHue::Violet, Scheme::Light) => [0x008B_5CF0, 0x006B_3FCC, 0x00EC_E4FB],
            (LabelHue::Red, Scheme::Dark) => [0x00FF_6B60, 0x00FF_8A80, 0x003A_1E1C],
            (LabelHue::Amber, Scheme::Dark) => [0x00F5_BC4E, 0x00F0_B84A, 0x003A_2C12],
            (LabelHue::Green, Scheme::Dark) => [0x004F_D06A, 0x006E_DB86, 0x0015_3520],
            (LabelHue::Blue, Scheme::Dark) => [0x004C_9BFF, 0x006F_B0FF, 0x0014_263F],
            (LabelHue::Violet, Scheme::Dark) => [0x00A9_8BFF, 0x00B6_9BFF, 0x0027_1C42],
        }
    }
}
