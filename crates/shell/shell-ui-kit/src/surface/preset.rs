//! The layer configurations the shell's surfaces share. A surface names a [`LayerPreset`] and
//! the preset fills in the ten fields of a `LayerOptions`, so a sheet is three lines and the
//! blur radius is one number ([`PANEL_RADIUS`]), not a copy per surface.

use layer_shell_types::{
    Anchor, BlurSpec, ElementId, ExclusiveZone, KeyboardMode, LayerExtent, LayerLevel,
    LayerOptions, LayerSize, Margin, OutputInfo, OutputTarget, RegionSpec,
};

use super::namespace;
use crate::panel::PANEL_RADIUS;

/// An element of the surface's document the compositor blurs behind and clips input to: the
/// `id` of the element and its corner.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PanelBlur {
    pub id: &'static str,
    pub radius: i32,
}

impl PanelBlur {
    /// A sheet's panel: rounded as the UI kit's `.ds-sheet` (`--r-panel`).
    #[must_use]
    pub const fn sheet(id: &'static str) -> PanelBlur {
        PanelBlur {
            id,
            radius: PANEL_RADIUS,
        }
    }

    /// The region that follows the element.
    #[must_use]
    pub fn region(self) -> RegionSpec {
        RegionSpec::Element {
            id: ElementId::new(self.id),
            radius: self.radius,
            inflate: 0,
        }
    }
}

/// Whether a surface that covers the output takes the pointer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Pointer {
    /// A press anywhere on it is the surface's.
    Takes,
    /// It takes nothing: every pointer event passes to what is under it.
    PassesThrough,
}

/// The shapes the shell's layer surfaces come in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LayerPreset {
    /// Overlay over the whole output, keyboard `Exclusive`, input everywhere (a press away from
    /// the panel is the surface's), blur behind `blur` only, or none.
    FullOutputSheet { blur: Option<PanelBlur> },
    /// As the sheet, but input and blur on `panel` only, so a click beside it reaches the window
    /// below.
    PanelOnly { panel: PanelBlur },
    /// Across the top of the output, `height` px tall, reserving nothing, no keyboard, input on
    /// `bar` only.
    Strip { height: u32, bar: PanelBlur },
    /// A `width` by `height` px column in the `anchor` corner of the output, no keyboard, input
    /// and blur on `card`.
    Corner {
        anchor: Anchor,
        width: u32,
        height: u32,
        exclusive: ExclusiveZone,
        card: PanelBlur,
    },
    /// The whole output at `level`, no keyboard, no blur.
    Whole { level: LayerLevel, pointer: Pointer },
}

impl LayerPreset {
    /// Where the surface takes the pointer: its configuration's `input`.
    #[must_use]
    pub fn input(self) -> RegionSpec {
        match self {
            LayerPreset::FullOutputSheet { .. } => RegionSpec::Whole,
            LayerPreset::PanelOnly { panel } => panel.region(),
            LayerPreset::Strip { bar, .. } => bar.region(),
            LayerPreset::Corner { card, .. } => card.region(),
            LayerPreset::Whole { pointer, .. } => match pointer {
                Pointer::Takes => RegionSpec::Whole,
                Pointer::PassesThrough => RegionSpec::Empty,
            },
        }
    }

    /// The configuration of the surface called `name` on `output`.
    #[must_use]
    pub fn config(self, name: &str, output: &OutputInfo) -> LayerOptions {
        let full = LayerSize {
            width: LayerExtent::Fill,
            height: LayerExtent::Fill,
        };
        let base = LayerOptions {
            namespace: namespace(name, output),
            layer: LayerLevel::Overlay,
            anchor: Anchor::ALL,
            size: full,
            exclusive: ExclusiveZone::Ignore,
            margin: Margin::ZERO,
            keyboard: KeyboardMode::None,
            output: OutputTarget::Output(output.id),
            blur: BlurSpec::None,
            input: self.input(),
            opaque: false,
        };
        match self {
            LayerPreset::FullOutputSheet { blur } => LayerOptions {
                keyboard: KeyboardMode::Exclusive,
                blur: blur.map_or(BlurSpec::None, |panel| BlurSpec::Region(panel.region())),
                ..base
            },
            LayerPreset::PanelOnly { panel } => LayerOptions {
                keyboard: KeyboardMode::Exclusive,
                blur: BlurSpec::Region(panel.region()),
                ..base
            },
            LayerPreset::Strip { height, .. } => LayerOptions {
                layer: LayerLevel::Top,
                anchor: Anchor::TOP | Anchor::LEFT | Anchor::RIGHT,
                size: LayerSize {
                    width: LayerExtent::Fill,
                    height: LayerExtent::Px(height),
                },
                ..base
            },
            LayerPreset::Corner {
                anchor,
                width,
                height,
                exclusive,
                card,
            } => LayerOptions {
                anchor,
                size: LayerSize {
                    width: LayerExtent::Px(width),
                    height: LayerExtent::Px(height),
                },
                exclusive,
                blur: BlurSpec::Region(card.region()),
                ..base
            },
            LayerPreset::Whole { level, .. } => LayerOptions {
                layer: level,
                ..base
            },
        }
    }
}
