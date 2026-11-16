//! `control_center.*` (design/22 §3.13; design/13
//! §13.3.7). All Advanced (§5).

use serde::{Deserialize, Serialize};
use ui_core::word::Word;
use ui_settings::schema::Page;

/// One control center module, in `control_center.modules` order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Word)]
#[serde(rename_all = "snake_case")]
#[word(case = snake)]
pub enum ControlCenterModule {
    Wifi,
    Bluetooth,
    Focus,
    Display,
    /// The keyboard's backlight (design/26 G25); drawn only where the machine has one.
    KeyboardBrightness,
    Sound,
    NowPlaying,
    Appearance,
    Battery,
    /// What the terminal may do without asking, and the way back from a pause (intentd's
    /// `Control`); drawn only while it has a grant or a pause to show.
    Terminal,
}

/// The control center popover.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ui_settings::SettingsSchema)]
#[serde(default)]
#[settings(file = "shell/settings.toml", domain = "control_center", page = Page::Appearance)]
#[expect(
    clippy::struct_excessive_bools,
    reason = "a settings table: every bool is its own key the person can flip, and no state machine hides in them"
)]
pub struct ControlCenterSettings {
    #[settings(
        label = "Modules",
        help = "Which tiles show, and in what order.",
        section = "Control Center",
        advanced
    )]
    pub modules: Vec<ControlCenterModule>,
    /// Whether Wi-Fi also has its own bar item and dropdown (macOS's "Show in Menu Bar").
    #[settings(
        label = "Wi-Fi in the menu bar",
        help = "Shows Wi-Fi as its own menu bar item.",
        section = "Menu bar",
        advanced
    )]
    pub menu_bar_wifi: bool,
    /// Whether Bluetooth also has its own bar item and dropdown (macOS's "Show in Menu Bar").
    #[settings(
        label = "Bluetooth in the menu bar",
        help = "Shows Bluetooth as its own menu bar item.",
        section = "Menu bar",
        advanced
    )]
    pub menu_bar_bluetooth: bool,
    /// Whether Sound also has its own bar item and dropdown (macOS's "Show in Menu Bar").
    #[settings(
        label = "Sound in the menu bar",
        help = "Shows Sound as its own menu bar item.",
        section = "Menu bar",
        advanced
    )]
    pub menu_bar_sound: bool,
    /// Whether Display also has its own bar item and dropdown (macOS's "Show in Menu Bar").
    #[settings(
        label = "Display in the menu bar",
        help = "Shows Display as its own menu bar item.",
        section = "Menu bar",
        advanced
    )]
    pub menu_bar_display: bool,
    /// Whether Battery also has its own bar item and dropdown (macOS's "Show in Menu Bar").
    #[settings(
        label = "Battery in the menu bar",
        help = "Shows Battery as its own menu bar item.",
        section = "Menu bar",
        advanced
    )]
    pub menu_bar_battery: bool,
    /// Whether Now Playing also has its own bar item and dropdown (macOS's "Show in Menu Bar").
    #[settings(
        label = "Now Playing in the menu bar",
        help = "Shows Now Playing as its own menu bar item.",
        section = "Menu bar",
        advanced
    )]
    pub menu_bar_now_playing: bool,
}

impl Default for ControlCenterSettings {
    fn default() -> Self {
        use ControlCenterModule::{
            Appearance, Battery, Bluetooth, Display, Focus, KeyboardBrightness, NowPlaying, Sound,
            Terminal, Wifi,
        };
        ControlCenterSettings {
            modules: vec![
                Wifi,
                Bluetooth,
                Focus,
                Display,
                KeyboardBrightness,
                Sound,
                NowPlaying,
                Appearance,
                Battery,
                Terminal,
            ],
            menu_bar_wifi: true,
            menu_bar_bluetooth: false,
            menu_bar_sound: true,
            menu_bar_display: false,
            menu_bar_battery: true,
            menu_bar_now_playing: false,
        }
    }
}
