//! `shell/settings.toml`, whole (design/22 §2): a [`SettingsDoc`] that `settings_store_fs::Store` reads
//! leniently (a bad value costs its own key), writes atomically and watches.
//!
//! A key this build does not know (a removed setting included) is reported in `Loaded::unknown` and
//! kept in the file by the next save; `shell_app::daemon::settings` owns the reading, saving and
//! watching.

use serde::{Deserialize, Serialize};
use settings_store::{FileName, Format, SettingsDoc};
use settings_types::AppName;

use crate::bar::BarSettings;
use crate::calendar::CalendarSettings;
use crate::companion::CompanionSettings;
use crate::control_center::ControlCenterSettings;
use crate::display::DisplaySettings;
use crate::dock::DockSettings;
use crate::hot_corners::HotCornerSettings;
use crate::idle::IdleSettings;
use crate::keyboard::KeyboardSettings;
use crate::launcher::LauncherSettings;
use crate::notifications::NotificationsSettings;
use crate::screenshot::ScreenshotSettings;
use crate::scroll::ScrollSettings;
use crate::scrollbar::ScrollbarSettings;
use crate::session::SessionSettings;
use crate::voice::VoiceSettings;
use crate::wallpaper::WallpaperSettings;

/// The directory name under `$XDG_CONFIG_HOME`.
pub const APP: AppName = AppName::desktop("shell");

/// Every shell domain (design/22 §4.4 `ShellFile`).
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct ShellFile {
    pub bar: BarSettings,
    pub dock: DockSettings,
    pub launcher: LauncherSettings,
    pub scroll: ScrollSettings,
    pub scrollbar: ScrollbarSettings,
    pub notifications: NotificationsSettings,
    pub control_center: ControlCenterSettings,
    pub display: DisplaySettings,
    /// bardo-shell's own lock screen.
    pub session: SessionSettings,
    pub calendar: CalendarSettings,
    pub screenshot: ScreenshotSettings,
    pub hot_corners: HotCornerSettings,
    /// bardo-shell's own idle service.
    pub idle: IdleSettings,
    /// How the keyboard is treated per app (keycap profiles).
    pub keyboard: KeyboardSettings,
    /// The companion's summon and confirmation.
    pub companion: CompanionSettings,
    /// Hold-to-talk and its consent.
    pub voice: VoiceSettings,
    /// The desktop's picture.
    pub wallpaper: WallpaperSettings,
}

impl SettingsDoc for ShellFile {
    const FILE: FileName = FileName("settings.toml");
    const FORMAT: Format = Format::Toml;
}
