//! The one level control the Sound and Display modules share.
//!
//! Bardo's `ModulePanel` headed by the title alone, as on the Mac (no glyph, no percentage: the
//! knob carries the glyph and the slider says its value to assistive technology), over Bardo's
//! `Slider` in its `Mac` look: the Control Center's slider, a Regular well (22) under a 20 knob
//! that carries the level's glyph, click to jump, drag the knob, arrows to step (Bardo design/29
//! §16). The look is chosen here and nowhere else ([`LOOK`]). The OSD draws the same level through
//! Bardo's `Osd`, as the bezel's sixteen squares.

use dioxus::prelude::*;
use shell_kit::prelude::*;
use ui_components::components::content::level_glyph::vocab::LevelSource;
use ui_components::components::controls::slider_model::SliderLook;
use ui_components::prelude::*;

use super::modules::use_plate;

/// The control center's level look.
const LOOK: SliderLook = SliderLook::Level;

/// `glyph` and `title` over the level at `level` (the percentage as the header's value), the
/// knob carrying `level_glyph` (the sun, or the volume's state, the bar item's source);
/// `on_change` hears every new level from a drag or a key. `Disabled` shows the level and takes
/// nothing: Bardo dims the level and the panel's header with it, so both get it.
#[component]
pub fn LevelModule(
    #[props(into)] level_glyph: LevelSource,
    title: String,
    level: Fraction,
    on_change: EventHandler<Fraction>,
    #[props(default)] availability: Availability,
) -> Element {
    rsx! {
        ModulePanel {
            title: TextLine::from(&title),
            plate: use_plate(),
            availability,
            Slider {
                label: title,
                value: level,
                glyph: level_glyph,
                look: LOOK,
                availability,
                onchange: move |value: Fraction| on_change.call(value),
            }
        }
    }
}
