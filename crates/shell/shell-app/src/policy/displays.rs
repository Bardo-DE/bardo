//! The displays service's policy: the `display.*` settings as the service's [`DisplayPolicy`]
//! (shell-services may not see shell-settings, which brings dioxus).

use shell_model::displays::{DisplayPolicy, DisplaysCommand};

use super::Policy;
use shell_settings::{DisplaySettings, ShellFile};

/// `display.*` as the service's policy.
#[must_use]
pub fn policy_from(settings: &DisplaySettings) -> DisplayPolicy {
    DisplayPolicy {
        keys_target: settings.brightness_keys_target,
        hardware_volume: settings.hardware_volume,
        night_warmth: settings.night_warmth,
        night_warmth_strength: shell_model::units::Percent(u16::from(
            settings.night_warmth_strength.0,
        )),
        font_rendering: settings.font_rendering,
    }
}

/// The displays service's policy from `display.*`.
#[derive(Debug, Clone, Copy)]
pub struct DisplaysBridge;

impl Policy for DisplaysBridge {
    type Command = DisplaysCommand;
    type Out = DisplayPolicy;

    fn of(&self, file: &ShellFile) -> DisplayPolicy {
        policy_from(&file.display)
    }

    fn command(out: DisplayPolicy) -> DisplaysCommand {
        DisplaysCommand::Policy(out)
    }
}

#[cfg(test)]
mod tests {
    use shell_model::displays::{
        DisplayPolicy, FontRenderingMode, HardwareVolume, KeysTarget, NightWarmthMode,
    };
    use shell_model::units::Percent;
    use shell_settings::DisplaySettings;

    use super::policy_from;

    #[test]
    fn the_default_settings_are_the_default_policy() {
        assert_eq!(
            policy_from(&DisplaySettings::default()),
            DisplayPolicy::default()
        );
    }

    #[test]
    fn every_setting_reaches_the_policy() {
        let settings = DisplaySettings {
            brightness_keys_target: KeysTarget::AllOutputs,
            hardware_volume: HardwareVolume::Off,
            night_warmth: NightWarmthMode::Hardware,
            night_warmth_strength: shell_settings::Percent(30),
            font_rendering: FontRenderingMode::Off,
        };
        assert_eq!(
            policy_from(&settings),
            DisplayPolicy {
                keys_target: KeysTarget::AllOutputs,
                hardware_volume: HardwareVolume::Off,
                night_warmth: NightWarmthMode::Hardware,
                night_warmth_strength: Percent(30),
                font_rendering: FontRenderingMode::Off,
            }
        );
    }
}
