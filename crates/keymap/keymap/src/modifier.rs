//! The modifiers a chord holds, as keyboard-types' [`Modifiers`] flags.
//!
//! A toolkit's flags may hold anything (locks, AltGr, Fn); a chord holds only Ctrl, Alt, Shift
//! and the OS key, which keyboard-types calls `META` (Command, the Windows key, Super).
//! [`chord_modifiers`] is that narrowing.

use keyboard_types::{Modifiers, NamedKey};

/// The modifiers a chord can hold, in the order sets are walked.
pub const CHORD_MODIFIERS: [Modifiers; 4] = [
    Modifiers::CONTROL,
    Modifiers::ALT,
    Modifiers::SHIFT,
    Modifiers::META,
];

/// `held` as a chord holds it: every flag other than Ctrl, Alt, Shift and the OS key (`META`)
/// is dropped.
pub(crate) const fn chord_modifiers(held: Modifiers) -> Modifiers {
    held.intersection(
        Modifiers::CONTROL
            .union(Modifiers::ALT)
            .union(Modifiers::SHIFT)
            .union(Modifiers::META),
    )
}

/// The modifier key `key` is.
pub(crate) const fn modifier_of_key(key: NamedKey) -> Option<Modifiers> {
    match key {
        NamedKey::Control => Some(Modifiers::CONTROL),
        NamedKey::Alt => Some(Modifiers::ALT),
        NamedKey::Shift => Some(Modifiers::SHIFT),
        NamedKey::Meta => Some(Modifiers::META),
        _ => None,
    }
}

/// The key of a chord modifier, for a lone tap of it; `None` unless `modifier` is one of
/// [`CHORD_MODIFIERS`].
pub(crate) fn key_of_modifier(modifier: Modifiers) -> Option<NamedKey> {
    match modifier {
        m if m == Modifiers::CONTROL => Some(NamedKey::Control),
        m if m == Modifiers::ALT => Some(NamedKey::Alt),
        m if m == Modifiers::SHIFT => Some(NamedKey::Shift),
        m if m == Modifiers::META => Some(NamedKey::Meta),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flags_other_than_the_chord_modifiers_are_dropped() {
        let held = Modifiers::META | Modifiers::CAPS_LOCK | Modifiers::NUM_LOCK | Modifiers::SHIFT;
        assert_eq!(chord_modifiers(held), Modifiers::META | Modifiers::SHIFT);
        assert_eq!(chord_modifiers(Modifiers::ALT_GRAPH), Modifiers::empty());
    }

    #[test]
    fn modifier_keys_and_flags_map_both_ways() {
        for modifier in CHORD_MODIFIERS {
            let key = key_of_modifier(modifier).unwrap_or_else(|| panic!("{modifier:?}"));
            assert_eq!(modifier_of_key(key), Some(modifier));
        }
        assert_eq!(key_of_modifier(Modifiers::CONTROL | Modifiers::ALT), None);
        assert_eq!(modifier_of_key(NamedKey::Tab), None);
    }
}
