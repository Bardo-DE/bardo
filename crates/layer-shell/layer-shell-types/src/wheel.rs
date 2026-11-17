//! A wheel or touchpad as a component hears it (`use_wheel`): the `wl_pointer` axis frame reduced to
//! a source, a phase and a distance.

use crate::geometry::Point;
use crate::modifiers::Modifiers;

/// What turned: the compositor's `wl_pointer.axis_source`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum WheelSource {
    /// A wheel's detents (a tilt wheel's too; a compositor that names no source is taken as
    /// a wheel).
    Wheel,
    /// Fingers on a touchpad: they have a beginning and a lift.
    Finger,
    /// A continuous motion with no fingers, such as a pointing stick or button scrolling.
    Continuous,
}

/// Where in its run a wheel event is. Only a source with a beginning and an end (`Finger`,
/// `Continuous`) goes through `Began` and `Ended`; a wheel reports `Changed` only.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum WheelPhase {
    /// The first motion of a run.
    Began,
    /// Motion within a run, and every wheel detent.
    Changed,
    /// The fingers lifted (`wl_pointer.axis_stop`); this event may carry a last motion.
    Ended,
}

/// How far the content moves, in logical px. Fractional because a touchpad's motion is; the
/// only float in the public vocabulary of input.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct WheelMotion {
    /// Positive to the right: the direction the content travels, not the offset's.
    pub x: f64,
    /// Positive down.
    pub y: f64,
}

/// One `wl_pointer.frame`'s scroll: all its axes, source and stop grouped as the compositor
/// grouped them.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WheelEvent {
    pub source: WheelSource,
    pub phase: WheelPhase,
    /// A wheel detent is the host's wheel step (60 px by default), a touchpad's px are its own.
    pub by: WheelMotion,
    /// The pointer, in the surface's logical px.
    pub at: Point,
    /// The modifiers held: a viewer zooms with the wheel under Control and scrolls otherwise.
    pub held: Modifiers,
}
