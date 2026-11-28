//! The desktop-entry layer every program shares: where entries live, the desktop id of a file, the
//! match of a window's app id to an entry, and the per-app identity record an entry declares.
//!
//! One answer to each question, so the dock, Mission Control and the launcher cannot disagree:
//!
//! - [`XdgEnv::applications_dirs`]: the folders to search, most important first;
//! - [`DesktopId`]: the id a file's place gives it (`kde/foo.desktop` is `kde-foo.desktop`, as the
//!   specification says);
//! - [`entry_for_window`]: the entry a window belongs to, by the five rules of [`match_app`];
//! - [`AppIdentity`]: the names one app is known by, read from `X-Bardo-` keys of its entry.
//!
//! Everything here is pure. `xdg-fs` walks the folders and reads the files.

mod data_dirs;
mod desktop_id;
mod entry;
mod identity;
mod match_app;

pub use data_dirs::XdgEnv;
pub use desktop_id::{DesktopId, DesktopIdError};
pub use entry::{ApplicationEntry, entry_for_window};
pub use identity::AppIdentity;
pub use match_app::{AppKeys, match_app};
