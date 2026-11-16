//! The launcher surfaces: the panel (rows, preview pane, actions menu) and its click catcher,
//! over `shell-launcher`'s core and `shell-search`'s results.

pub mod launcher;

pub use launcher::Launcher;
pub use launcher::LauncherCatcher;
