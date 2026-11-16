//! The vocabulary of symbol effects (design/35-SYMBOL-EFFECTS.md): which motion an icon plays and
//! when it runs, as one enum a caller writes and a table the stylesheet and the part driver read.
//!
//! Each effect has one run mode, and the type says which: a `Once` effect plays when its
//! [`Epoch`] changes, a `While` effect plays for as long as its flag is `true`, and a
//! `Transition` effect plays when its icon arrives, leaves, draws or is replaced.

use crate::epoch::Epoch;
use ui_core::word::Word;

/// The effects that play once and settle: a hop, a dip, a few swings, a swell, a turn, the
/// icon's own moving part, the layers lighting in order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Word)]
pub enum OnceEffect {
    /// The symbol hops and settles.
    Bounce,
    /// The symbol dims and returns.
    Pulse,
    /// The symbol turns a few degrees each way, each smaller.
    Wiggle,
    /// The symbol swells and settles.
    Breathe,
    /// The symbol turns once about its centre.
    Rotate,
    /// The layers of the icon light in order (Wi-Fi bars, volume waves, battery cells); an icon
    /// with no layers pulses.
    VariableColor,
    /// The icon's own moving part plays its gesture (a lid lifts, a bell rings); an icon with no
    /// annotated part bounces.
    Part,
}

/// The effects that run for as long as they are active.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Word)]
pub enum LoopEffect {
    /// The symbol bounces again and again.
    Bounce,
    /// The symbol dims and returns, again and again.
    Pulse,
    /// The symbol is held larger.
    ScaleUp,
    /// The symbol is held smaller.
    ScaleDown,
    /// The symbol wiggles again and again.
    Wiggle,
    /// The symbol swells and settles, again and again.
    Breathe,
    /// The symbol turns at a constant speed.
    Rotate,
    /// The layers of the icon light in order, then again.
    VariableColor,
    /// The icon's own moving part plays its gesture again and again.
    Part,
}

/// The effects that bring a symbol in, take it out, draw it or swap it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Word)]
pub enum TransitionEffect {
    /// The symbol grows in from a smaller size as it fades up, when it becomes visible.
    Appear,
    /// The symbol fades out, when it becomes hidden.
    Disappear,
    /// The symbol's strokes draw along their paths when visible, and back off when hidden.
    DrawOn,
    /// The symbol cross-fades into the icon it is given next; the visibility is not read.
    Replace,
}

/// One symbol effect and the run mode that fires it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SymbolEffect {
    /// Plays once each time the trigger changes; the first render plays nothing.
    Once(OnceEffect, Epoch),
    /// Plays for as long as the flag is `true`.
    While(LoopEffect, bool),
    /// Plays when the visibility, or for `Replace` the icon, changes; the first render stands at
    /// the given state.
    Transition(TransitionEffect, bool),
}

impl SymbolEffect {
    /// The effect that plays nothing: a symbol whose motion is driven another way renders it.
    pub const NONE: SymbolEffect = SymbolEffect::Transition(TransitionEffect::Replace, true);

    /// The effect's name, `data-symbol`.
    pub fn slug(self) -> &'static str {
        match self {
            SymbolEffect::Once(effect, _) => effect.slug(),
            SymbolEffect::While(effect, _) => effect.slug(),
            SymbolEffect::Transition(effect, _) => effect.slug(),
        }
    }

    /// How it runs, `data-run`.
    pub fn run(self) -> &'static str {
        match self {
            SymbolEffect::Once(..) => "once",
            SymbolEffect::While(..) => "while",
            SymbolEffect::Transition(..) => "transition",
        }
    }
}
