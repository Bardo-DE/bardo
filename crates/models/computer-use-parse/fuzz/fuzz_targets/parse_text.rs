//! `parse_text` is total: any bytes, any space, any limits, no panic, and the result is within
//! the limits it was given.
#![no_main]

use computer_use_geometry::{GridMax, ModelSpace};
use computer_use_parse::{InSpace, ParseLimits, parse_text};
use libfuzzer_sys::fuzz_target;
use model_types::TextDialect;

fuzz_target!(|data: &[u8]| {
    let text = String::from_utf8_lossy(data);
    let limits = ParseLimits::default();
    for space in [ModelSpace::Image, ModelSpace::Grid(GridMax(1000))] {
        if let Ok(parsed) = parse_text(TextDialect::UiTars15, space, &text, limits) {
            let kept = match &parsed.actions {
                InSpace::Image(actions) => actions.len(),
                InSpace::Grid(_, actions) => actions.len(),
            };
            assert!(kept <= usize::from(limits.max_actions.0));
        }
    }
});
