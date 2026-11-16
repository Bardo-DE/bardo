//! A pending loop's look as data (design/26-DETAILS.md section 3.2, R4; design/30 section 1.3):
//! how it moves and what it draws in a frame. The frame at a time is
//! [`crate::timeline::pending::Pending`].

/// A pending loop's look: how it moves, with what only that movement needs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PendingSpec {
    /// One layer at a time: the Wi-Fi bars searching.
    Iterate {
        /// Over how many layers (the Wi-Fi glyph: the dot and three arcs = 4).
        layers: u8,
    },
    /// A ring turns a twelfth per step: the spinner.
    Spin,
}

/// The steps of one turn of the spinner: twelve spokes.
pub const SPIN_STEPS: u8 = 12;

impl PendingSpec {
    /// Steps in one cycle: a layer each for Iterate, twelve twelfths for Spin.
    #[must_use]
    pub fn cycle(self) -> u8 {
        match self {
            PendingSpec::Iterate { layers } => layers.max(1),
            PendingSpec::Spin => SPIN_STEPS,
        }
    }
}

/// What to draw.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PendingFrame {
    /// No operation: draw the state as it is.
    Idle,
    /// The loop's `n`th step since it started, counted modulo [`SPIN_STEPS`] (0 first).
    /// [`PendingFrame::lit`] reads a layer.
    Step(u8),
}

impl PendingFrame {
    /// The `data-pending` word: `idle` or `step`.
    #[must_use]
    pub fn slug(self) -> &'static str {
        match self {
            PendingFrame::Idle => "idle",
            PendingFrame::Step(_) => "step",
        }
    }

    /// Whether `layer` (0 first) is drawn in this frame of `spec`: Idle draws every layer;
    /// Iterate lights one layer a step; Spin is always On (it turns instead).
    #[must_use]
    pub fn lit(self, spec: PendingSpec, layer: u8) -> bool {
        let PendingFrame::Step(n) = self else {
            return true;
        };
        match spec {
            PendingSpec::Iterate { .. } => layer == n % spec.cycle(),
            PendingSpec::Spin => true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{PendingFrame, PendingSpec};

    #[test]
    fn each_style_lights_its_own_layers() {
        const ON: bool = true;
        const OFF: bool = false;

        let lit =
            |frame: PendingFrame, spec| (0..4).map(|l| frame.lit(spec, l)).collect::<Vec<_>>();
        let wifi = PendingSpec::Iterate { layers: 4 };
        assert_eq!(lit(PendingFrame::Step(2), wifi), [OFF, OFF, ON, OFF]);
        assert_eq!(lit(PendingFrame::Step(5), wifi), [OFF, ON, OFF, OFF]);
        assert_eq!(lit(PendingFrame::Idle, wifi), [ON, ON, ON, ON]);
        assert_eq!(PendingFrame::Step(7).lit(PendingSpec::Spin, 0), ON);
    }
}
