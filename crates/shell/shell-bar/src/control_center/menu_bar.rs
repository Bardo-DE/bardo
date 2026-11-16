//! Which modules also sit in the bar as their own items (macOS's "Show in Menu Bar",
//! `control_center.menu_bar_*`), and in what order, pure. Each such item opens a dropdown
//! holding the same module component the control center shows (`PopupBody::Module`).

use shell_settings::{ControlCenterModule, ControlCenterSettings};

use super::modules::module;

/// The modules that can be bar items, left to right as the bar draws them: the status items
/// the bar always had keep their order (Sound, then Wi-Fi, then Battery), the new ones join
/// around them, and the control center's own item and the clock follow at the right end.
pub const MENU_BAR_ORDER: [ControlCenterModule; 6] = [
    ControlCenterModule::NowPlaying,
    ControlCenterModule::Display,
    ControlCenterModule::Sound,
    ControlCenterModule::Bluetooth,
    ControlCenterModule::Wifi,
    ControlCenterModule::Battery,
];

/// `kind`'s `control_center.menu_bar_*` key; `false` for a module that cannot be a bar item
/// (Focus, Appearance).
#[must_use]
pub fn in_menu_bar(cc: &ControlCenterSettings, kind: ControlCenterModule) -> bool {
    module(kind).menu_bar(cc)
}

/// The modules shown as bar items, in [`MENU_BAR_ORDER`].
#[must_use]
pub fn menu_bar_modules(cc: &ControlCenterSettings) -> Vec<ControlCenterModule> {
    MENU_BAR_ORDER
        .into_iter()
        .filter(|&module| in_menu_bar(cc, module))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{in_menu_bar, menu_bar_modules};
    use shell_settings::{ControlCenterModule, ControlCenterSettings};

    #[test]
    fn wifi_sound_and_battery_show_by_default_in_bar_order() {
        use ControlCenterModule::{Appearance, Battery, Bluetooth, NowPlaying, Sound, Wifi};
        let cc = ControlCenterSettings::default();
        assert_eq!(menu_bar_modules(&cc), vec![Sound, Wifi, Battery]);
        assert!(!in_menu_bar(&cc, Appearance));
        let more = ControlCenterSettings {
            menu_bar_bluetooth: true,
            menu_bar_now_playing: true,
            menu_bar_sound: false,
            ..ControlCenterSettings::default()
        };
        assert_eq!(
            menu_bar_modules(&more),
            vec![NowPlaying, Bluetooth, Wifi, Battery]
        );
    }
}
