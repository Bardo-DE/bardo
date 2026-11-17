//! The seat's modifier state from the wire: sctk's decoded modifiers, and the DOM flags an injected
//! key carries. The types are `layer_shell_types::{Modifier, LockKey, Modifiers}`.

use layer_shell_types::{LockKey, Modifier, Modifiers};
use smithay_client_toolkit::seat::keyboard::Modifiers as SctkModifiers;

/// `Modifiers` from sctk's decoded modifier state: already resolved against the keymap, so
/// `shift`, `ctrl`, `alt` and `logo` are the currently-held (depressed or latched) modifiers, and
/// `caps_lock`/`num_lock` are the currently-locked ones.
#[must_use]
pub(crate) fn from_sctk(state: SctkModifiers) -> Modifiers {
    Modifiers {
        shift: Modifier::from(state.shift),
        ctrl: Modifier::from(state.ctrl),
        alt: Modifier::from(state.alt),
        logo: Modifier::from(state.logo),
        caps_lock: LockKey::from(state.caps_lock),
        num_lock: LockKey::from(state.num_lock),
    }
}

/// `held` with the four held modifiers taken from the DOM flags `mods`, the locks kept: what an
/// injected key's modifier change comes to, in place of a `wl_keyboard.modifiers`.
pub(crate) fn with_dom(held: Modifiers, mods: keyboard_types::Modifiers) -> Modifiers {
    let of = |flag| Modifier::from(mods.contains(flag));
    Modifiers {
        shift: of(keyboard_types::Modifiers::SHIFT),
        ctrl: of(keyboard_types::Modifiers::CONTROL),
        alt: of(keyboard_types::Modifiers::ALT),
        logo: Modifier::from(
            mods.intersects(keyboard_types::Modifiers::SUPER | keyboard_types::Modifiers::META),
        ),
        ..held
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn state(f: impl FnOnce(&mut SctkModifiers)) -> SctkModifiers {
        let mut s = SctkModifiers::default();
        f(&mut s);
        s
    }

    #[test]
    fn each_field_maps_to_its_own_enum() {
        let cases: Vec<(&str, SctkModifiers, Modifiers)> = vec![
            ("none", state(|_| {}), Modifiers::default()),
            (
                "shift held",
                state(|s| s.shift = true),
                Modifiers {
                    shift: Modifier::Held,
                    ..Modifiers::default()
                },
            ),
            (
                "ctrl held",
                state(|s| s.ctrl = true),
                Modifiers {
                    ctrl: Modifier::Held,
                    ..Modifiers::default()
                },
            ),
            (
                "caps lock locked is a lock, not a held modifier",
                state(|s| s.caps_lock = true),
                Modifiers {
                    caps_lock: LockKey::Locked,
                    ..Modifiers::default()
                },
            ),
            (
                "alt held",
                state(|s| s.alt = true),
                Modifiers {
                    alt: Modifier::Held,
                    ..Modifiers::default()
                },
            ),
            (
                "logo held",
                state(|s| s.logo = true),
                Modifiers {
                    logo: Modifier::Held,
                    ..Modifiers::default()
                },
            ),
            (
                "num lock locked",
                state(|s| s.num_lock = true),
                Modifiers {
                    num_lock: LockKey::Locked,
                    ..Modifiers::default()
                },
            ),
            (
                "everything at once",
                state(|s| {
                    s.ctrl = true;
                    s.shift = true;
                    s.logo = true;
                    s.caps_lock = true;
                }),
                Modifiers {
                    shift: Modifier::Held,
                    ctrl: Modifier::Held,
                    logo: Modifier::Held,
                    caps_lock: LockKey::Locked,
                    ..Modifiers::default()
                },
            ),
        ];
        for (name, input, want) in cases {
            assert_eq!(from_sctk(input), want, "{name}");
        }
    }
}
