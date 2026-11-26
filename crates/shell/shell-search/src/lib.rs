//! The launcher's search: the runner that queries every provider off the UI thread and merges
//! their batches (`shell_model::results::ResultsService`, run here), its history and frecency, and
//! the providers themselves.
//!
//! This crate never reaches `dioxus` (`cargo xtask check layers`): the launcher reads the
//! results through `use_service::<ResultsService>()`, and `shell-app` starts it.
#![expect(
    missing_docs,
    reason = "lints baseline: this crate documents its public items when its domain is refactored (plan phases 3-7)"
)]

mod clock;
pub mod providers;
pub mod results;
