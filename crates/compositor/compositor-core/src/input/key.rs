//! Key and button codes.

/// An evdev key code as the seat protocols carry it (`u32`, and any value may arrive): which
/// key was pressed, before any layout. `key_types::KeyCode` is the named `u16` vocabulary the
/// kernel devices use.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct KeyCode(pub u32);

/// An evdev button code (`BTN_LEFT` is `0x110`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ButtonCode(pub u32);
