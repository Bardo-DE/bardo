//! The one key vocabulary: evdev key codes with their names, the four chord modifiers and sets of
//! them, and XKB keysyms with their names and the logical keys they mean.
//!
//! Pure data and tables, no I/O. The logical key vocabulary stays `keyboard_types::Key`; the
//! maps here (`KeyCode::dom_code`, `Keysym::logical_key`, `Keysym::location`) lead to it.

mod code;
mod dom;
mod keysym;
mod logical;
mod modifier;
mod names;

pub use code::KeyCode;
pub use keysym::Keysym;
pub use modifier::{Modifier, Modifiers};
