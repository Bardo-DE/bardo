//! A DOM key as the center's [`CenterKey`] (design/20 §1.6: arrows move, Return opens or
//! expands, Delete clears, Escape closes; Backspace is the Mac keyboard's Delete). Pure.

use dioxus::prelude::{Key, NamedKey};

use super::keys::CenterKey;

/// `key`, when the center handles it.
#[must_use]
pub fn center_key(key: &Key) -> Option<CenterKey> {
    match key {
        Key::Named(NamedKey::Escape) => Some(CenterKey::Escape),
        Key::Named(NamedKey::ArrowUp) => Some(CenterKey::Up),
        Key::Named(NamedKey::ArrowDown) => Some(CenterKey::Down),
        Key::Named(NamedKey::Enter) => Some(CenterKey::Return),
        Key::Named(NamedKey::Delete) | Key::Named(NamedKey::Backspace) => Some(CenterKey::Delete),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::center_key;
    use crate::notification_center::CenterKey;
    use dioxus::prelude::{Key, NamedKey};

    #[test]
    fn keys_map_per_design_20_section_1_6() {
        let cases = [
            (Key::Named(NamedKey::Escape), Some(CenterKey::Escape)),
            (Key::Named(NamedKey::ArrowUp), Some(CenterKey::Up)),
            (Key::Named(NamedKey::ArrowDown), Some(CenterKey::Down)),
            (Key::Named(NamedKey::Enter), Some(CenterKey::Return)),
            (Key::Named(NamedKey::Delete), Some(CenterKey::Delete)),
            (Key::Named(NamedKey::Backspace), Some(CenterKey::Delete)),
            (Key::Named(NamedKey::ArrowLeft), None),
            (Key::Named(NamedKey::Tab), None),
            (Key::Character("x".to_owned()), None),
        ];
        for (key, want) in cases {
            assert_eq!(center_key(&key), want, "{key:?}");
        }
    }
}
