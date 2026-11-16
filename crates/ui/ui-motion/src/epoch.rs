//! A wrapping counter: the caller's way to say "play again" to a component that otherwise moves
//! only on mount and on a change of its own value (an animated emoji's awake window,
//! design/25-EMOJI.md; a battery ring's fill, design/23-WIDGETS.md section 4.1; a symbol's
//! `Once` effect, design/35-SYMBOL-EFFECTS.md).

/// A counter: pass a different value (`Epoch::next`) to fire a `Once` symbol effect or to wake
/// a component, as a lock screen does when the pointer moves or a key is pressed, or a widget
/// host does when its widgets come into view. The same value again fires nothing; mounting
/// wakes a component too.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Epoch(pub u32);

impl Epoch {
    /// The epoch one step later.
    pub fn next(self) -> Epoch {
        Epoch(self.0.wrapping_add(1))
    }
}
