//! Keeping the mini window above the others, where the desktop lets a program do that.

/// Ask for a stacking order.
pub trait WindowStacking {
    /// Ask for the mini window to stay above the others when `keep_above`, else to be a normal
    /// window. `true` when the window is stacked as asked; `false` when the desktop gives no way
    /// to ask and the window stays a normal one.
    fn request(&self, keep_above: bool) -> bool;
}
