//! The modifier keys held by the seat's keyboard, published to whichever surface has keyboard
//! focus (`use_modifiers`). Distinct from `input::modifiers::modifiers_from`, which
//! feeds Blitz's own key events their `keyboard_types::Modifiers` bitflags unchanged: this is the
//! typed, no-`bool` shape the rest of the host's public surface uses.

/// Whether a modifier key is held down right now (pressed or latched).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Modifier {
    #[default]
    Released,
    Held,
}

/// The wire form: sctk reports a held modifier as a bare `bool`.
impl From<bool> for Modifier {
    fn from(wire: bool) -> Modifier {
        if wire {
            Modifier::Held
        } else {
            Modifier::Released
        }
    }
}

/// Whether a lock key's toggle is engaged. Distinct from [`Modifier`]: Caps Lock and Num Lock
/// are xkb "lock" modifiers, switched by a tap of the key rather than held for the duration of
/// an effect.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum LockKey {
    #[default]
    Unlocked,
    Locked,
}

/// The wire form: sctk reports an engaged lock as a bare `bool`.
impl From<bool> for LockKey {
    fn from(wire: bool) -> LockKey {
        if wire {
            LockKey::Locked
        } else {
            LockKey::Unlocked
        }
    }
}

/// The keyboard's modifiers, as of the focused surface's last `wl_keyboard.modifiers`.
///
/// The compositor sends one right after `wl_keyboard.enter`, and again whenever the state changes
/// with no key event, such as a bare Shift press. Empty for a surface that has never had keyboard
/// focus, and reset to empty when it loses focus (`use_modifiers`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Modifiers {
    pub shift: Modifier,
    pub ctrl: Modifier,
    pub alt: Modifier,
    pub logo: Modifier,
    pub caps_lock: LockKey,
    pub num_lock: LockKey,
}
