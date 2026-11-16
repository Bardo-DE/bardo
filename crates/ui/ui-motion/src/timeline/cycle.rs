//! A share going round: from nothing to whole once, or from nothing to whole again and again for as long as it is let run (design/35-SYMBOL-EFFECTS.md section 3).
//!
//! The frame is how far along the curve one cycle is, in thousandths; a timeline at rest stands at
//! 1000, the end of a cycle.

use super::Timeline;
use super::ease::Ease;
use std::time::Duration;
use ui_style::tokens::easing::Easing;

/// How many times a cycle runs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Repeat {
    /// Once, then it stands at the end.
    Once,
    /// Again and again; it never settles.
    Forever,
}

/// A cycle `period` long along `easing`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Cycle {
    /// How long one cycle takes; none at all is a cycle at its end.
    pub period: Duration,
    /// The curve of one cycle.
    pub easing: Easing,
    /// Whether it goes round again.
    pub repeat: Repeat,
}

impl Cycle {
    /// A cycle that stands at its end.
    pub const REST: Cycle = Cycle {
        period: Duration::ZERO,
        easing: Easing::Linear,
        repeat: Repeat::Once,
    };
}

impl Timeline for Cycle {
    type Frame = i64;

    fn total(&self) -> Duration {
        match self.repeat {
            Repeat::Once => self.period,
            Repeat::Forever => Duration::MAX,
        }
    }

    fn at(&self, elapsed: Duration) -> i64 {
        let into = match self.repeat {
            Repeat::Forever if !self.period.is_zero() => Duration::from_nanos(
                u64::try_from(elapsed.as_nanos() % self.period.as_nanos()).unwrap_or(u64::MAX),
            ),
            _ => elapsed,
        };
        let ease = Ease {
            length: self.period,
            easing: self.easing,
        };
        i64::from(ease.at(into).eased.0)
    }
}

#[cfg(test)]
mod tests {
    use super::{Cycle, Repeat, Timeline};
    use std::time::Duration;
    use ui_style::tokens::easing::Easing;

    const MS: fn(u64) -> Duration = Duration::from_millis;

    #[test]
    fn a_cycle_runs_once_and_stands_at_its_end() {
        const CASES: &[(u64, i64)] = &[(0, 0), (200, 500), (400, 1000), (9_000, 1000)];

        let once = Cycle {
            period: MS(400),
            easing: Easing::Linear,
            repeat: Repeat::Once,
        };
        // (elapsed ms, frame): the start, the middle, the end and past it.
        for &(ms, want) in CASES {
            assert_eq!(once.at(MS(ms)), want, "{ms} ms");
        }
        assert!(once.settled(MS(400)));
        assert_eq!(Cycle::REST.at(Duration::ZERO), 1000);
    }

    #[test]
    fn a_repeating_cycle_goes_round_and_never_settles() {
        const CASES: &[(u64, i64)] = &[(0, 0), (250, 250), (1_000, 0), (1_500, 500), (7_250, 250)];

        let forever = Cycle {
            period: MS(1000),
            easing: Easing::Linear,
            repeat: Repeat::Forever,
        };
        for &(ms, want) in CASES {
            assert_eq!(forever.at(MS(ms)), want, "{ms} ms");
        }
        assert!(!forever.settled(MS(3_600_000)));
    }
}
