//! One opening's machine: quire's switcher ([`ui_behaviour::switcher::Phase`], the one the
//! compositor runs too) with the one fact only this surface knows. Whether the show delay's wake
//! ends a tap depends on what the modifiers have told the opening (`hold.rs`), so the surface
//! provides that beside the app list as the machine's context at every step, including the step
//! the delay's wake causes.

use ui_behaviour::switcher::{Phase, SwIn, SwOut, SwitcherParams};
use ui_components::base::machine::Machine;
use ui_components::base::time::stamp::Stamp;

use super::hold::{Hold, tick_input};

/// The switcher over the apps by index.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Opening(Phase);

impl Opening {
    /// An opening in `phase`.
    pub fn in_phase(phase: Phase) -> Opening {
        Opening(phase)
    }

    /// Where it is.
    pub fn phase(&self) -> Phase {
        self.0
    }
}

/// What a step reads from outside: how many MRU apps there are (the current one first), and what
/// the modifiers have said.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Facts {
    pub apps: usize,
    pub hold: Hold,
}

impl Machine for Opening {
    type In = SwIn;
    type Out = SwOut;
    type Params = SwitcherParams;
    type Ctx = Facts;

    fn step(
        self,
        input: SwIn,
        at: Stamp,
        params: &SwitcherParams,
        cx: &Facts,
    ) -> (Opening, Vec<SwOut>) {
        let input = match input {
            SwIn::Elapsed => tick_input(self.phase(), cx.hold),
            other => other,
        };
        let (next, outs) = self.0.step(input, at, params, &cx.apps);
        (Opening(next), outs)
    }

    fn wake(&self) -> Option<Stamp> {
        self.0.wake()
    }
}
