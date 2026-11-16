//! One window's target.

use super::ConfigureNeed;
use compositor_core::geometry::LogicalRect;
use compositor_core::{OutputKey, WindowId};

/// Where a window goes, and whether its client must be asked for a new size.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WindowTarget {
    /// The window.
    pub window: WindowId,
    /// The output it is arranged on (motion and the renderer group by it).
    pub output: OutputKey,
    /// Its rectangle, in global logical coordinates.
    pub rect: LogicalRect,
    /// Whether to configure it.
    pub size: ConfigureNeed,
}
