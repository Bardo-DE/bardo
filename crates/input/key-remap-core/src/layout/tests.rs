use super::*;
use crate::event::Physical;
use crate::role::Role;
use key_types::KeyCode;

#[test]
fn role_by_position() {
    const CASES: &[(Layout, KeyCode, Option<Role>)] = &[
        (Layout::Apple, KeyCode::LEFTMETA, Some(Role::Command)),
        (Layout::Apple, KeyCode::RIGHTMETA, Some(Role::Command)),
        (Layout::Apple, KeyCode::LEFTALT, Some(Role::Option)),
        (Layout::Apple, KeyCode::RIGHTALT, Some(Role::Option)),
        (Layout::Pc, KeyCode::LEFTALT, Some(Role::Command)),
        (Layout::Pc, KeyCode::RIGHTALT, Some(Role::Command)),
        (Layout::Pc, KeyCode::LEFTMETA, Some(Role::Option)),
        (Layout::Pc, KeyCode::RIGHTMETA, Some(Role::Option)),
        (Layout::Apple, KeyCode::LEFTCTRL, Some(Role::Control)),
        (Layout::Apple, KeyCode::RIGHTCTRL, Some(Role::Control)),
        (Layout::Pc, KeyCode::LEFTCTRL, Some(Role::Control)),
        (Layout::Pc, KeyCode::RIGHTCTRL, Some(Role::Control)),
        (Layout::Apple, KeyCode::LEFTSHIFT, Some(Role::Shift)),
        (Layout::Apple, KeyCode::RIGHTSHIFT, Some(Role::Shift)),
        (Layout::Pc, KeyCode::LEFTSHIFT, Some(Role::Shift)),
        (Layout::Pc, KeyCode::RIGHTSHIFT, Some(Role::Shift)),
        (Layout::Apple, KeyCode::ESC, None),
        (Layout::Pc, KeyCode::ESC, None),
    ];
    for &(layout, key, want) in CASES {
        assert_eq!(
            role(layout, Physical(key)),
            want,
            "{layout:?} {}",
            key.name()
        );
    }
}

#[test]
fn unnamed_codes_are_not_modifiers() {
    assert_eq!(
        role(Layout::Apple, Physical(KeyCode::from_code(0xffff))),
        None
    );
}

#[test]
fn layout_of_vendor_table() {
    const CASES: &[(u16, Layout)] = &[
        (0x05ac, Layout::Apple),
        (0x004c, Layout::Apple),
        (0x3434, Layout::Pc),
        (0x046d, Layout::Pc),
        (0x0000, Layout::Pc),
    ];
    for &(vendor, want) in CASES {
        assert_eq!(Layout::of_vendor(vendor), want, "{vendor:#06x}");
    }
}
