//! The launcher's search: the runner that queries every provider off the UI thread and merges
//! their batches (`shell_model::results::ResultsService`, run here), its history and frecency, and
//! the providers themselves.
//!
//! This crate never reaches `dioxus` (`cargo xtask check layers`): the launcher reads the
//! results through `use_service::<ResultsService>()`, and `shell-app` starts it.

mod clock;
pub mod providers;
pub mod results;
