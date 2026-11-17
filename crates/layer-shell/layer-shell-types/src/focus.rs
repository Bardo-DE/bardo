//! Keyboard focus, as a surface's document and the shell see it change.

/// Keyboard focus, gained or lost.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum KeyboardFocus {
    Gained,
    Lost,
}
