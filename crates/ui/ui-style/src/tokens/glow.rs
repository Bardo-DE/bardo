//! The glow the compositor draws around the window the companion acts in (design/32 section 3).
//!
//! The UI kit owns the values and the compositor links this crate to read them, so the glow it draws
//! is the one the design system describes; this module is plain data and names no renderer.

use ui_core::geometry::units::Px;

/// A spot inside the window the glow brightens, in logical pixels.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GlowSpot {
    /// Across, from the window's left edge.
    pub x: Px,
    /// Down, from the window's top edge.
    pub y: Px,
    /// How far the spot reaches.
    pub radius: Px,
}

/// What the glow shows.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum GlowLook {
    /// No glow.
    None,
    /// Working out of sight: a soft ring.
    Working,
    /// Acting in the window, brightest at the spot where it acts.
    Acting {
        /// Where it acts, when it knows.
        spot: Option<GlowSpot>,
    },
    /// Waiting for the person: still.
    Waiting,
}
