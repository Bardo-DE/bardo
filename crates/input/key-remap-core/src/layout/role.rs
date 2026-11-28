//! `role`: what a physical key means as a modifier on a layout.

use super::Layout;
use crate::event::Physical;
use crate::role::Role;
use key_types::KeyCode;

/// The role of `key` on `layout`, or `None` if it is not a modifier.
///
/// By position: the key beside the space bar is Command on both layouts (Apple: Meta, PC: Alt)
/// and the next one out is Option (Apple: Alt, PC: Meta).
#[must_use]
pub fn role(layout: Layout, key: Physical) -> Option<Role> {
    let (beside_space, next_out) = match layout {
        Layout::Apple => (
            [KeyCode::LEFTMETA, KeyCode::RIGHTMETA],
            [KeyCode::LEFTALT, KeyCode::RIGHTALT],
        ),
        Layout::Pc => (
            [KeyCode::LEFTALT, KeyCode::RIGHTALT],
            [KeyCode::LEFTMETA, KeyCode::RIGHTMETA],
        ),
    };
    let key = key.0;
    if beside_space.contains(&key) {
        Some(Role::Command)
    } else if next_out.contains(&key) {
        Some(Role::Option)
    } else if [KeyCode::LEFTCTRL, KeyCode::RIGHTCTRL].contains(&key) {
        Some(Role::Control)
    } else if [KeyCode::LEFTSHIFT, KeyCode::RIGHTSHIFT].contains(&key) {
        Some(Role::Shift)
    } else {
        None
    }
}
