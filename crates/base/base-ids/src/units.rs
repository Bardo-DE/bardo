//! Units and counts: every number that means something has a type.

use serde::{Deserialize, Serialize};

macro_rules! unit {
    ($(#[$doc:meta])* $name:ident($inner:ty)) => {
        $(#[$doc])*
        #[derive(
            Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Default, Serialize, Deserialize,
        )]
        #[serde(transparent)]
        pub struct $name(pub $inner);
    };
}

unit!(
    /// A number of model tokens (context window, output limit, usage).
    Tokens(u32)
);
unit!(
    /// The length of an embedding vector.
    Dims(u32)
);
unit!(
    /// Image pixels along one side.
    Px(u32)
);
unit!(
    /// A plain count of things (documents, users, languages).
    Count(u32)
);
unit!(
    /// A size in bytes.
    Bytes(u64)
);
unit!(
    /// A number of turns (replies a provider gave, whatever they cost).
    TurnCount(u32)
);
unit!(
    /// Thousandths of a whole: thresholds such as the spend warning at 800.
    Permille(u32)
);

/// A proportion in thousandths: 0 is none, 1000 is all.
///
/// Not clamped by the type: a settings gain may exceed 1000; a consumer that draws it clamps it
/// with [`Fraction::clamped`].
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Default, Serialize, Deserialize,
)]
#[serde(transparent)]
pub struct Fraction(pub u16);

impl Fraction {
    /// All of it.
    const ONE: Fraction = Fraction(1000);

    /// The value clamped to 0..=1000.
    #[must_use]
    pub fn clamped(self) -> Self {
        Fraction(self.0.min(Self::ONE.0))
    }

    /// The value in whole percent, rounded: what a battery's figure prints and its
    /// `aria-valuenow` reads.
    #[must_use]
    pub fn whole_percent(self) -> u16 {
        (self.clamped().0 + 5) / 10
    }

    /// The clamped value as a CSS number for `--f`: `0.35`, `0`, `1`.
    #[must_use]
    pub fn css(self) -> String {
        let permille = self.clamped().0;
        let text = format!("{}.{:03}", permille / 1000, permille % 1000);
        text.trim_end_matches('0').trim_end_matches('.').to_string()
    }
}

#[cfg(test)]
mod fraction_tests {
    use super::Fraction;

    #[test]
    fn a_fraction_reads_as_whole_percent_rounded_and_clamped() {
        const CASES: &[(u16, u16)] = &[
            (0, 0),
            (4, 0),
            (5, 1),
            (844, 84),
            (845, 85),
            (1000, 100),
            (1200, 100),
        ];
        for &(given, want) in CASES {
            assert_eq!(Fraction(given).whole_percent(), want, "{given}");
        }
    }
}
