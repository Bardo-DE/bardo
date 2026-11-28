//! The four modifier keys a chord names, and sets of them.

use crate::code::KeyCode;

/// A modifier key, always written as its left-hand key.
///
/// The declaration order (Ctrl, Alt, Shift, Super) is the order sets are walked and sorted in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
pub enum Modifier {
    /// Control.
    Ctrl,
    /// Alt (Option on a Mac keyboard).
    Alt,
    /// Shift.
    Shift,
    /// Super (Command on a Mac keyboard, the Windows key elsewhere).
    Super,
}

impl Modifier {
    /// Every modifier, in declaration order.
    pub const ALL: [Modifier; 4] = [
        Modifier::Ctrl,
        Modifier::Alt,
        Modifier::Shift,
        Modifier::Super,
    ];

    /// The lowercase word a config or a settings file uses for this modifier.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Modifier::Ctrl => "ctrl",
            Modifier::Alt => "alt",
            Modifier::Shift => "shift",
            Modifier::Super => "super",
        }
    }

    /// The modifier `name` stands for (exactly the words [`Modifier::name`] gives).
    #[must_use]
    pub fn named(name: &str) -> Option<Modifier> {
        Modifier::ALL.into_iter().find(|m| m.name() == name)
    }

    /// The left-hand key written for this modifier.
    #[must_use]
    pub const fn key(self) -> KeyCode {
        match self {
            Modifier::Ctrl => KeyCode::LEFTCTRL,
            Modifier::Alt => KeyCode::LEFTALT,
            Modifier::Shift => KeyCode::LEFTSHIFT,
            Modifier::Super => KeyCode::LEFTMETA,
        }
    }

    const fn bit(self) -> u8 {
        1 << self as u8
    }
}

/// A set of modifiers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct Modifiers(u8);

impl Modifiers {
    /// The set with no members.
    pub const EMPTY: Modifiers = Modifiers(0);

    /// This set plus `member`.
    #[must_use]
    pub const fn with(self, member: Modifier) -> Modifiers {
        Modifiers(self.0 | member.bit())
    }

    /// This set minus `member`.
    #[must_use]
    pub const fn without(self, member: Modifier) -> Modifiers {
        Modifiers(self.0 & !member.bit())
    }

    /// Whether `member` is in the set.
    #[must_use]
    pub const fn contains(self, member: Modifier) -> bool {
        self.0 & member.bit() != 0
    }

    /// Whether the set has no members.
    #[must_use]
    pub const fn is_empty(self) -> bool {
        self.0 == 0
    }

    /// Everything in `self` or `other`.
    #[must_use]
    pub const fn union(self, other: Modifiers) -> Modifiers {
        Modifiers(self.0 | other.0)
    }

    /// Everything in `self` that is not in `other`.
    #[must_use]
    pub const fn difference(self, other: Modifiers) -> Modifiers {
        Modifiers(self.0 & !other.0)
    }

    /// The members, in declaration order.
    pub fn iter(self) -> impl Iterator<Item = Modifier> {
        Modifier::ALL.into_iter().filter(move |m| self.contains(*m))
    }
}

impl FromIterator<Modifier> for Modifiers {
    fn from_iter<I: IntoIterator<Item = Modifier>>(iter: I) -> Self {
        iter.into_iter().fold(Modifiers::EMPTY, Modifiers::with)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_and_keys() {
        const CASES: &[(&str, Option<(Modifier, KeyCode)>)] = &[
            ("ctrl", Some((Modifier::Ctrl, KeyCode::LEFTCTRL))),
            ("alt", Some((Modifier::Alt, KeyCode::LEFTALT))),
            ("shift", Some((Modifier::Shift, KeyCode::LEFTSHIFT))),
            ("super", Some((Modifier::Super, KeyCode::LEFTMETA))),
            ("command", None),
            ("control", None),
            ("Ctrl", None),
        ];
        for &(name, want) in CASES {
            assert_eq!(Modifier::named(name), want.map(|(m, _)| m), "{name:?}");
            if let Some((m, key)) = want {
                assert_eq!(m.name(), name);
                assert_eq!(m.key(), key);
            }
        }
    }

    #[test]
    fn set_with_without_contains_and_order() {
        let held = Modifiers::EMPTY.with(Modifier::Super).with(Modifier::Ctrl);
        assert!(Modifiers::EMPTY.is_empty());
        assert!(!held.is_empty());
        assert!(held.contains(Modifier::Ctrl));
        assert!(held.contains(Modifier::Super));
        assert!(!held.contains(Modifier::Alt));
        assert_eq!(held.with(Modifier::Ctrl), held, "with is idempotent");
        assert_eq!(
            held.without(Modifier::Ctrl),
            Modifiers::EMPTY.with(Modifier::Super)
        );
        assert_eq!(
            held.iter().collect::<Vec<_>>(),
            [Modifier::Ctrl, Modifier::Super],
            "iteration is in declaration order"
        );
        let both: Modifiers = [Modifier::Shift, Modifier::Alt].into_iter().collect();
        assert_eq!(held.union(both).iter().count(), 4);
        assert_eq!(held.union(both).difference(both), held);
    }
}
