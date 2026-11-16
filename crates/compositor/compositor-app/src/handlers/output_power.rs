//! `zwlr_output_power_manager_v1` (P1-M1): `compositor_wayland::output_power` serves the
//! protocol; this file gives it the root's part, the output lookup, the model's power and the
//! loop. A request is an `Event::OutputRequested` that changes only the power.

use crate::state::State;
use compositor_core::OutputKey;
use compositor_wayland::output_power::{OutputPowerHandler, OutputPowerPart};

impl OutputPowerHandler for State {
    fn output_power_part(&mut self) -> &mut OutputPowerPart {
        &mut self.wl.output_power
    }

    fn output_power_of(&self, output: &OutputKey) -> Option<bool> {
        self.core
            .model
            .outputs()
            .0
            .get(output)
            .map(|plugged| plugged.config.power)
    }
}
