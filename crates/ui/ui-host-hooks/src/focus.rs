//! Where the keyboard goes after a click on nothing focusable.

/// Where the keyboard goes after a click on nothing focusable, for a window or a harness.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum FocusFallback {
    /// To the nearest focusable ancestor (a `tabindex`, or a natively focusable element), as
    /// in a browser; nowhere if there is none.
    #[default]
    Ancestor,
    /// Blitz's own: the focus is cleared.
    BlitzDefault,
}
