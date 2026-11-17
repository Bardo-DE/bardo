//! Keys as actions: the one toolkit integration of the chord keymap (the keymap core).
//!
//! A dioxus key event becomes a chord keymap key press in one place (`ui_core::command::key_input`),
//! the active keymap comes from an injected [`KeySource`], and a press resolves to an `Action` in
//! the right `Context`. Apps declare their own actions with a portable default chord and handle
//! actions; they never read modifiers or decide Command versus Ctrl. See CONSUMING.md "Keyboard:
//! actions, not modifiers".

mod bardo_actions;
mod handle;
mod on_action;
mod provider;
mod source;
mod state;

pub(crate) use bardo_actions::{pane_back as pane_back_action, register_bardo_actions};
pub use handle::Keys;
pub use on_action::on_action;
pub use provider::{use_keys, use_keys_provider, use_platform, use_register, use_register_actions};
pub use source::KeySource;

#[cfg(test)]
mod tests;
