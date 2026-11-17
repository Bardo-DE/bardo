//! The status tray: SNI items and their dbusmenu rows, as the bar draws them.

mod command;
mod icon_name;
mod label;
mod limits;
mod model;
mod step;
#[cfg(test)]
mod tests;

pub use command::{ScrollAxis, ScrollDelta, TrayCommand, TrayEvent};
pub use icon_name::theme_icon_name;
pub use label::plain_label;
pub use limits::{
    MENU_DEPTH_MAX, MENU_ICON_MAX_BYTES, MENU_LABEL_MAX_CHARS, MENU_ROWS_MAX, PIXMAP_MAX_SIDE,
    TRAY_ICON_FILE_MAX_BYTES, TRAY_ICON_NAME_MAX_BYTES, TRAY_ITEMS_MAX,
};
pub use model::{
    Check, Chord, MenuCommand, Pixmap, Tray, TrayIcon, TrayItem, TrayItemId, TrayMenuEntry,
    TrayMenuIcon, TrayMenuKind, TrayStatus,
};
pub use step::step;

/// The tray host as a service.
///
/// It registers a `StatusNotifierWatcher` when none is on the bus
/// (`system_tray::client::Client::new`'s own `StatusNotifierWatcher::attach_to`: it tries to own
/// `org.kde.StatusNotifierWatcher` and quietly defers when a desktop already does). Without a
/// session bus the state stays at `Tray::default()`.
#[derive(Debug)]
pub enum TrayService {}

impl crate::service::Service for TrayService {
    const NAME: &'static str = "tray";
    type State = Tray;
    type Command = TrayCommand;
}
