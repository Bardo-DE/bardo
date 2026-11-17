//! Layer-shell surfaces: what a shell asks for when it spawns a bar, a dock or an overlay.
//!
//! See FINDINGS F76.

use crate::blur::BlurSpec;
use crate::geometry::Margin;
use crate::id::OutputId;
use crate::region::RegionSpec;

/// Everything `HostCtx::spawn_layer` needs besides the root component.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LayerOptions {
    /// The layer-shell namespace (`shell-bar`, `shell-dock`); compositors key rules on it, and the
    /// debug commands address the surface by it.
    pub namespace: String,
    pub layer: LayerLevel,
    pub anchor: Anchor,
    pub size: LayerSize,
    pub exclusive: ExclusiveZone,
    pub margin: Margin,
    pub keyboard: KeyboardMode,
    pub output: OutputTarget,
    pub blur: BlurSpec,
    pub input: RegionSpec,
    /// Every pixel the surface draws is opaque: the host tells the compositor so (an opaque
    /// region over the whole surface) and asks for an opaque swapchain when the display offers
    /// one, so the compositor can skip what it covers. `false` for anything translucent.
    pub opaque: bool,
}

/// Which stacking layer a surface lives in, bottom to top.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LayerLevel {
    Background,
    /// Below normal windows: a dock that windows may cover (the occlusion check of render
    /// acceptance item c).
    Bottom,
    Top,
    Overlay,
}

/// The edges a layer surface is anchored to: a set of up to four edges. Hand-written rather than
/// `bitflags` because `bitflags` is not in the pinned block.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Anchor(u8);

impl Anchor {
    /// Anchored to nothing: centred on the output.
    pub const NONE: Anchor = Anchor(0);
    pub const TOP: Anchor = Anchor(1);
    pub const BOTTOM: Anchor = Anchor(2);
    pub const LEFT: Anchor = Anchor(4);
    pub const RIGHT: Anchor = Anchor(8);
    /// All four edges.
    pub const ALL: Anchor = Anchor(15);

    /// Both sets of edges.
    #[must_use]
    pub const fn union(self, other: Anchor) -> Anchor {
        Anchor(self.0 | other.0)
    }

    /// Whether every edge of `other` is in `self`.
    #[must_use]
    pub const fn contains(self, other: Anchor) -> bool {
        self.0 & other.0 == other.0
    }
}

impl std::ops::BitOr for Anchor {
    type Output = Anchor;

    fn bitor(self, rhs: Anchor) -> Anchor {
        self.union(rhs)
    }
}

/// One dimension of a layer surface's requested size.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LayerExtent {
    /// Stretch between the two opposite anchors (wire value 0; the protocol requires both
    /// opposite edges to be anchored, or the compositor raises an error).
    Fill,
    /// A fixed size in logical pixels.
    Px(u32),
}

/// The size a layer surface asks for, in logical pixels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct LayerSize {
    pub width: LayerExtent,
    pub height: LayerExtent,
}

/// How a layer surface interacts with other surfaces' exclusive zones.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ExclusiveZone {
    /// Extend under other panels (wire -1).
    Ignore,
    /// Keep clear of other panels but reserve nothing (wire 0).
    Auto,
    /// Reserve this many logical pixels along the anchored edge (a bar's 32).
    Reserve(u32),
}

/// When a layer surface receives keyboard focus.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum KeyboardMode {
    /// Never (bar, dock). A grabbing popup opened from such a surface switches it to `OnDemand`
    /// until the popup closes (plan: layer-shell-host input rule; design/13 §13.3.8), except on `KWin`,
    /// which focuses the popup itself (`popup_keyboard`).
    None,
    /// Takes all keyboard input while mapped (launcher, switcher).
    Exclusive,
    /// Focusable like a window.
    OnDemand,
}

/// Which output a layer surface goes on.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum OutputTarget {
    /// Let the compositor choose (usually the focused output).
    CompositorPicks,
    /// This output.
    Output(OutputId),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn anchor_set_operations() {
        let bar = Anchor::TOP | Anchor::LEFT | Anchor::RIGHT;
        assert!(bar.contains(Anchor::TOP));
        assert!(bar.contains(Anchor::LEFT | Anchor::RIGHT));
        assert!(!bar.contains(Anchor::BOTTOM));
        assert!(Anchor::ALL.contains(bar));
    }
}
