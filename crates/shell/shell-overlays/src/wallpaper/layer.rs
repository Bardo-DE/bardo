//! The wallpaper's layer surface: Background, every edge, ignoring exclusive zones, no keyboard,
//! input over the whole output (a press of the secondary button opens the desktop's menu).

use layer_shell_types::{LayerConfig, LayerLevel, OutputInfo};

use shell_ui_kit::surface::{LayerPreset, Pointer};

/// The wallpaper's layer configuration on `output`.
pub fn layer(output: &OutputInfo) -> LayerConfig {
    LayerConfig {
        // The picture sits on an opaque `--paper` ground over the whole surface (style.css), so
        // the compositor need not blend it or draw what it covers.
        opaque: true,
        ..LayerPreset::Whole {
            level: LayerLevel::Background,
            pointer: Pointer::Takes,
        }
        .config("wallpaper", output)
    }
}

#[cfg(test)]
mod tests {
    use super::layer;
    use layer_shell_types::{Anchor, BlurSpec, ExclusiveZone, LayerLevel, RegionSpec};
    use shell_ui_kit::harness::fake::output_named as output;

    #[test]
    fn the_wallpaper_fills_the_background_and_takes_the_pointer_for_its_menu() {
        let config = layer(&output(Some("DP-1")));
        assert_eq!(config.namespace, "shell-wallpaper-DP-1");
        assert_eq!(config.layer, LayerLevel::Background);
        assert_eq!(config.anchor, Anchor::ALL);
        assert_eq!(config.exclusive, ExclusiveZone::Ignore);
        assert_eq!(config.input, RegionSpec::Whole);
        assert_eq!(config.blur, BlurSpec::None);
        assert!(config.opaque);
    }
}
