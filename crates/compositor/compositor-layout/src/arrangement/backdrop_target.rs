//! The backdrop behind a Split View half that is narrower than its window's maximum.

use account_core::SpaceId;
use compositor_core::OutputKey;

/// A Split View Space where a window is centred in its half (it cannot grow to the half's
/// width): the rest of the output is drawn as a plain backdrop, as on the Mac.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BackdropTarget {
    /// The Split View Space.
    pub space: SpaceId,
    /// The output it is on.
    pub output: OutputKey,
}
