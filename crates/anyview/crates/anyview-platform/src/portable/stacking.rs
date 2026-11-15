//! Window stacking on Linux: a Wayland client cannot place itself above other windows, and the
//! compositor protocols that allow it are compositor-specific. This implementation says so.

use crate::stacking::WindowStacking;

/// [`WindowStacking`] for a desktop with no protocol to keep a window above: a normal window is
/// what it already is, and asking for more is refused.
#[derive(Debug, Clone, Copy)]
pub struct NoStacking;

impl WindowStacking for NoStacking {
    fn request(&self, keep_above: bool) -> bool {
        !keep_above
    }
}
