//! A DOM key as the panel's [`LauncherKey`] (design/06 §20.2): arrows, Enter, Tab and Shift+Tab,
//! Escape, and the shared action that opens the row's actions (`shell.show-actions`, ⌘K).
//!
//! The keymap resolves a key to an action first (`PanelCtx::resolved`); these read the action,
//! never a modifier that means Command. Pure.

use dioxus::prelude::{Key, Modifiers, NamedKey};
use keymap::{Action, StandardAction};
use shell_launcher::OwnAction;

use crate::launcher::companion::CompanionKey;
use crate::launcher::keys::LauncherKey;

/// `key` with `modifiers` held, when the panel handles it; `action` is what the keymap made of
/// the key.
pub fn launcher_key(
    key: &Key,
    modifiers: Modifiers,
    action: Option<&Action>,
) -> Option<LauncherKey> {
    match key {
        Key::Named(NamedKey::ArrowUp) => Some(LauncherKey::Up),
        Key::Named(NamedKey::ArrowDown) => Some(LauncherKey::Down),
        Key::Named(NamedKey::Enter) => Some(LauncherKey::Enter),
        Key::Named(NamedKey::Escape) => Some(LauncherKey::Escape),
        Key::Named(NamedKey::Tab) if modifiers.contains(Modifiers::SHIFT) => {
            Some(LauncherKey::ShiftTab)
        }
        Key::Named(NamedKey::Tab) => Some(LauncherKey::Tab),
        _ => action
            .and_then(OwnAction::of)
            .filter(|own| *own == OwnAction::ShowActions)
            .map(|_| LauncherKey::Actions),
    }
}

/// `key` with `modifiers` held, when it is one companion mode reads before the palette does.
///
/// Escape, and the actions that send on (`shell.promote`, ⌘↵), cancel (Cancel, ⌘.) and switch the
/// permissions mode (`shell.switch-mode`, ⇧⌘M).
#[must_use]
pub fn companion_key(
    key: &Key,
    modifiers: Modifiers,
    action: Option<&Action>,
) -> Option<CompanionKey> {
    match (key, action) {
        (Key::Named(NamedKey::Escape), _) if modifiers.is_empty() => Some(CompanionKey::Escape),
        (_, Some(Action::Standard(StandardAction::Cancel))) => Some(CompanionKey::CommandPeriod),
        (_, Some(action)) if OwnAction::of(action) == Some(OwnAction::Promote) => {
            Some(CompanionKey::CommandEnter)
        }
        (_, Some(action)) if OwnAction::of(action) == Some(OwnAction::SwitchMode) => {
            Some(CompanionKey::SwitchMode)
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::{companion_key, launcher_key};
    use crate::launcher::keys::LauncherKey;
    use dioxus::prelude::{Key, Modifiers, NamedKey};
    use keymap::{Action, StandardAction};
    use shell_launcher::OwnAction;

    type LauncherCase = (Key, Modifiers, Option<Action>, Option<LauncherKey>);

    fn own(action: OwnAction) -> Option<Action> {
        action.action().map(Action::App)
    }

    #[test]
    fn keys_map_per_design_06_section_20_2() {
        let k = |s: &str| Key::Character(s.to_owned());
        let none = Modifiers::empty();
        let show_actions = own(OwnAction::ShowActions);
        let other = own(OwnAction::FocusPane);
        let cases: Vec<LauncherCase> = vec![
            (
                Key::Named(NamedKey::ArrowUp),
                none,
                None,
                Some(LauncherKey::Up),
            ),
            (
                Key::Named(NamedKey::ArrowDown),
                none,
                None,
                Some(LauncherKey::Down),
            ),
            (
                Key::Named(NamedKey::Enter),
                none,
                None,
                Some(LauncherKey::Enter),
            ),
            (
                Key::Named(NamedKey::Escape),
                none,
                None,
                Some(LauncherKey::Escape),
            ),
            (
                Key::Named(NamedKey::Tab),
                none,
                None,
                Some(LauncherKey::Tab),
            ),
            (
                Key::Named(NamedKey::Tab),
                Modifiers::SHIFT,
                None,
                Some(LauncherKey::ShiftTab),
            ),
            // The keymap said the key opens the actions.
            (
                k("k"),
                Modifiers::META,
                show_actions,
                Some(LauncherKey::Actions),
            ),
            // Plain typing is the field's, and so is every other action.
            (k("k"), none, None, None),
            (k("k"), Modifiers::META, other, None),
            (Key::Named(NamedKey::Backspace), none, None, None),
        ];
        for (key, modifiers, action, want) in cases {
            assert_eq!(
                launcher_key(&key, modifiers, action.as_ref()),
                want,
                "{key:?} {modifiers:?}"
            );
        }
    }

    #[test]
    fn companion_keys_are_escape_and_the_three_actions() {
        use crate::launcher::companion::CompanionKey as K;
        type CompanionCase = (Key, Modifiers, Option<Action>, Option<K>);
        let k = |s: &str| Key::Character(s.to_owned());
        let none = Modifiers::empty();
        let promote = own(OwnAction::Promote);
        let switch_mode = own(OwnAction::SwitchMode);
        let cancel = Some(Action::Standard(StandardAction::Cancel));
        let cases: Vec<CompanionCase> = vec![
            (Key::Named(NamedKey::Escape), none, None, Some(K::Escape)),
            (Key::Named(NamedKey::Escape), Modifiers::SHIFT, None, None),
            (Key::Named(NamedKey::Enter), none, None, None),
            (
                Key::Named(NamedKey::Enter),
                Modifiers::META,
                promote,
                Some(K::CommandEnter),
            ),
            (k("."), Modifiers::META, cancel, Some(K::CommandPeriod)),
            (
                k("m"),
                Modifiers::META | Modifiers::SHIFT,
                switch_mode,
                Some(K::SwitchMode),
            ),
            (k("."), none, None, None),
            (k(","), Modifiers::META, None, None),
        ];
        for (key, modifiers, action, want) in cases {
            assert_eq!(
                companion_key(&key, modifiers, action.as_ref()),
                want,
                "{key:?} {modifiers:?}"
            );
        }
    }
}
