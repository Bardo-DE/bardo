//! The freedesktop.org file formats Bardo reads, as pure parsers: the text comes in, the parsed
//! value comes out, and nothing here reads a file or the environment.
//!
//! - [`DesktopFile`]: a `.desktop` file (or any file in that key file format) from its text;
//! - the desktop-entry layer ([`DesktopId`], [`XdgEnv`], [`ApplicationEntry`], [`match_app`],
//!   [`AppIdentity`]): where entries live, the id of one, the entry a window belongs to;
//! - [`mime`]: a file's type from its name, the apps that open it and its icon name, per
//!   shared-mime-info, `mimeapps.list` and `mimeinfo.cache`;
//! - [`OsRelease`]: what `/etc/os-release` says about the distro.
//!
//! `xdg-fs` reads the files and the process environment these parsers take as arguments.

mod desktop_entries;
mod desktop_file;
pub mod mime;
mod os_release;

pub use desktop_entries::{
    AppIdentity, AppKeys, ApplicationEntry, DesktopId, DesktopIdError, XdgEnv, entry_for_window,
    match_app,
};
pub use desktop_file::{
    DESKTOP_ENTRY_CAP, DesktopFile, DesktopFileError, DesktopGroup, EntryLocale,
};
pub use os_release::OsRelease;
