//! Input roles by position: where on the keyboard a modifier key sits.

mod bitset;
mod input;
#[cfg(test)]
mod tests;

pub use input::{Role, Roles};
