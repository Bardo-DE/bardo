//! The `zwlr_layer_surface_v1` wire values each layer-surface choice becomes. The choices
//! themselves are `layer_shell_types`'s.
//!
//! See FINDINGS F76.

use layer_shell_types::{Anchor, ExclusiveZone, KeyboardMode, LayerExtent, LayerLevel, LayerSize};
use smithay_client_toolkit::shell::wlr_layer::{
    Anchor as WireAnchor, KeyboardInteractivity, Layer as WireLayer,
};

/// The sctk value for `layer`.
pub(crate) fn to_wire_layer(layer: LayerLevel) -> WireLayer {
    match layer {
        LayerLevel::Background => WireLayer::Background,
        LayerLevel::Bottom => WireLayer::Bottom,
        LayerLevel::Top => WireLayer::Top,
        LayerLevel::Overlay => WireLayer::Overlay,
    }
}

/// The sctk value for `anchor`.
pub(crate) fn to_wire_anchor(anchor: Anchor) -> WireAnchor {
    [
        (Anchor::TOP, WireAnchor::TOP),
        (Anchor::BOTTOM, WireAnchor::BOTTOM),
        (Anchor::LEFT, WireAnchor::LEFT),
        (Anchor::RIGHT, WireAnchor::RIGHT),
    ]
    .into_iter()
    .filter(|&(ours, _)| anchor.contains(ours))
    .fold(WireAnchor::empty(), |wire, (_, bit)| wire | bit)
}

/// The `set_size` arguments for `size`.
pub(crate) fn to_wire_size(size: LayerSize) -> (u32, u32) {
    let wire = |extent| match extent {
        LayerExtent::Fill => 0,
        LayerExtent::Px(px) => px,
    };
    (wire(size.width), wire(size.height))
}

/// The `set_exclusive_zone` argument for `zone`.
pub(crate) fn to_wire_exclusive(zone: ExclusiveZone) -> i32 {
    match zone {
        ExclusiveZone::Ignore => -1,
        ExclusiveZone::Auto => 0,
        ExclusiveZone::Reserve(px) => i32::try_from(px).unwrap_or(i32::MAX),
    }
}

/// The sctk value for `mode`.
pub(crate) fn to_wire_keyboard(mode: KeyboardMode) -> KeyboardInteractivity {
    match mode {
        KeyboardMode::None => KeyboardInteractivity::None,
        KeyboardMode::Exclusive => KeyboardInteractivity::Exclusive,
        KeyboardMode::OnDemand => KeyboardInteractivity::OnDemand,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_layer_maps_to_its_wire_layer() {
        let cases = [
            (LayerLevel::Background, WireLayer::Background),
            (LayerLevel::Bottom, WireLayer::Bottom),
            (LayerLevel::Top, WireLayer::Top),
            (LayerLevel::Overlay, WireLayer::Overlay),
        ];
        for (ours, wire) in cases {
            assert_eq!(to_wire_layer(ours), wire, "{ours:?}");
        }
    }

    #[test]
    fn anchors_map_edge_by_edge() {
        let cases = [
            ("none", Anchor::NONE, WireAnchor::empty()),
            (
                "bar",
                Anchor::TOP | Anchor::LEFT | Anchor::RIGHT,
                WireAnchor::TOP | WireAnchor::LEFT | WireAnchor::RIGHT,
            ),
            ("dock", Anchor::BOTTOM, WireAnchor::BOTTOM),
            ("all", Anchor::ALL, WireAnchor::all()),
        ];
        for (name, ours, wire) in cases {
            assert_eq!(to_wire_anchor(ours), wire, "{name}");
        }
    }

    #[test]
    fn sizes_zones_and_keyboard_modes_map_to_wire_values() {
        let sizes = [
            (
                LayerSize {
                    width: LayerExtent::Fill,
                    height: LayerExtent::Px(32),
                },
                (0, 32),
            ),
            (
                LayerSize {
                    width: LayerExtent::Px(740),
                    height: LayerExtent::Px(96),
                },
                (740, 96),
            ),
        ];
        for (size, wire) in sizes {
            assert_eq!(to_wire_size(size), wire, "{size:?}");
        }
        let zones = [
            (ExclusiveZone::Ignore, -1),
            (ExclusiveZone::Auto, 0),
            (ExclusiveZone::Reserve(32), 32),
            (ExclusiveZone::Reserve(u32::MAX), i32::MAX),
        ];
        for (zone, wire) in zones {
            assert_eq!(to_wire_exclusive(zone), wire, "{zone:?}");
        }
        let modes = [
            (KeyboardMode::None, KeyboardInteractivity::None),
            (KeyboardMode::Exclusive, KeyboardInteractivity::Exclusive),
            (KeyboardMode::OnDemand, KeyboardInteractivity::OnDemand),
        ];
        for (mode, wire) in modes {
            assert_eq!(to_wire_keyboard(mode), wire, "{mode:?}");
        }
    }
}
