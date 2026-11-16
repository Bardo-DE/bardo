//! Which outputs casement is told to leave alone: the monitors bardo-shell warms itself.

use std::collections::BTreeSet;

use crate::displays::NightWarmthMode;
use crate::displays::ddc::DdcDisplay;
use crate::ids::{DdcId, OutputName};

/// The connectors casement's night light must not warm (`night_light.skip_outputs`).
///
/// Where bardo-shell warms the monitors itself, each one that answers DDC/CI with RGB gains (the
/// ones bardo-shell's warmth writes reach) and whose connector is known; otherwise none. Sorted,
/// each connector once.
#[must_use]
pub fn skip_outputs(backend: NightWarmthMode, ddc: &[DdcDisplay]) -> Vec<OutputName> {
    match backend {
        NightWarmthMode::Off => Vec::new(),
        NightWarmthMode::Hardware => ddc
            .iter()
            .filter(|display| display.controls.gain.is_some())
            .filter_map(|display| display.endpoint.connector.clone())
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect(),
    }
}

/// The monitors bardo-shell warms whose connector it could not match: casement's software light warms
/// them too (it cannot be told to skip what bardo-shell cannot name). Sorted by bus.
#[must_use]
pub fn unmatched(backend: NightWarmthMode, ddc: &[DdcDisplay]) -> Vec<DdcId> {
    match backend {
        NightWarmthMode::Off => Vec::new(),
        NightWarmthMode::Hardware => ddc
            .iter()
            .filter(|display| display.controls.gain.is_some())
            .filter(|display| display.endpoint.connector.is_none())
            .map(|display| display.endpoint.id)
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect(),
    }
}
