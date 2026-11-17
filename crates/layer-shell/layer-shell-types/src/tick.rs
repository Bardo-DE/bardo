//! The frame tick a component sees (`use_frame_tick`).

use std::time::Duration;

/// One frame of a surface that is animating or driven: when it was called back, and how long
/// since the previous tick of the same run (zero for the first).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct FrameTick {
    /// Time since the host connected: the timeline every document's CSS is resolved on.
    pub at: Duration,
    pub dt: Duration,
}
