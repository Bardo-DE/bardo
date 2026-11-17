//! The installed apps: desktop entries parsed, their exec lines expanded, and the index the
//! launcher and dock match against (through `ui_helpers::match_app`).

mod command;
mod exec;
mod icon_file;
mod index;
mod renames;

pub use command::AppsCommand;
pub use exec::{ExecArgs, ExecError, expand_exec};
pub use icon_file::{ICON_FILE_MAX_BYTES, SVG_FILE_MAX_BYTES, icon_file_fits};
pub use index::{
    App, AppIcon, AppIndex, AppsEvent, DbusActivation, DesktopActionEntry, Listing, Terminal, step,
};
pub use renames::Renames;

/// A minimal entry with `id` and `name`, for the tests of what reads an index.
#[cfg(test)]
pub(crate) use index::tests::app as test_app;

/// The desktop entries and their icons as a service: scanned once and published, then every
/// applications directory watched and rescanned after half a second of quiet.
#[derive(Debug)]
pub enum AppsService {}

impl crate::service::Service for AppsService {
    const NAME: &'static str = "apps";
    type State = AppIndex;
    type Command = AppsCommand;
}
