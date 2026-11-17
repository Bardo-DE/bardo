//! The accounts sheet (design/31 §4.5 and §5.6, PLAN §2.4): accountd's consent question and its
//! add-account steps, drawn by bardo-shell on `org.bardo.AccountsSheet1`.
//!
//! A centred window of its own on the sheets' output, with a titlebar that carries the step's
//! title, over an Overlay layer that takes the keyboard exclusively; the steps inside are Bardo's
//! `ui-shell::accounts` parts, given the props `props.rs` makes of accountd's `SheetView`.
//!
//! Only accountd may open or drive it (`shell_services::accounts::sheet`); the typed values live in
//! the service's draft, and a secret leaves it with the one reply that carries it.
//!
//! Like the polkit prompt and the confirmation sheet this is phase A: a layer surface on a stock
//! compositor, not unforgeable. Phase B moves it to compositor-agent's trusted surface.

mod continue_as;
mod events;
mod io;
mod props;
mod ranking;
mod stage;
mod surface;
#[cfg(test)]
mod tests;
mod view;
mod words;

pub use surface::AccountsSheet;

/// The sheet's stylesheet (lint-clean under `Profile::Strict`, `crates/shell/shell-app/tests/lint.rs`).
pub const ACCOUNTS_SHEET_CSS: &str = include_str!("style.css");
