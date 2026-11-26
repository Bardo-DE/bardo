//! The launcher surfaces: the panel (rows, preview pane, actions menu) and its click catcher,
//! over `shell-launcher`'s core and `shell-search`'s results.
#![expect(
    missing_docs,
    reason = "lints baseline: this crate documents its public items when its domain is refactored (plan phases 3-7)"
)]

pub mod launcher;

pub use launcher::Launcher;
pub use launcher::LauncherCatcher;
