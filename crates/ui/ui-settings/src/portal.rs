//! The desktop's preferences from the settings portal (`settings-portal`), mapped to
//! [`ui_style::appearance::system::SystemSettings`]. The portal answers plain values; this is the
//! adapter to the toolkit's types. A desktop without the portal, or a build for a platform or
//! feature set that has none, answers [`SystemSettings::default`].

use base_spawn::Spawner;
use settings_portal::PortalPrefs;
use ui_style::appearance::system::{Contrast, SystemSettings};
use ui_style::appearance::theme::Scheme;

/// The toolkit's form of what the portal answered.
fn settings_from(prefs: PortalPrefs) -> SystemSettings {
    SystemSettings::default()
        .with_scheme(if prefs.dark {
            Scheme::Dark
        } else {
            Scheme::Light
        })
        .with_reduced_motion(prefs.reduced_motion)
        .with_contrast(if prefs.high_contrast {
            Contrast::High
        } else {
            Contrast::Normal
        })
}

/// Where the desktop's preferences come from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SystemSettingsSource {
    /// The settings portal on the session bus. A desktop without it, or a build without the portal client answers
    /// the defaults and never changes.
    Portal,
    /// These preferences, forever: a test, a headless render or a pinned appearance.
    Fixed(SystemSettings),
}

impl SystemSettingsSource {
    /// Every appearance preference, once. Never fails: a missing bus, portal or namespace is the
    /// defaults.
    pub async fn read(&self) -> SystemSettings {
        match self {
            SystemSettingsSource::Fixed(prefs) => *prefs,
            SystemSettingsSource::Portal => settings_from(settings_portal::read().await),
        }
    }
}

/// A subscription to the desktop's appearance preferences.
#[derive(Debug)]
pub struct SystemSettingsWatch {
    inner: Inner,
}

#[derive(Debug)]
enum Inner {
    /// A fixed answer: it never fires.
    Fixed(SystemSettings),
    Portal(settings_portal::Watch),
}

impl SystemSettingsWatch {
    /// Subscribe to `source` and then read it, the subscription's task running on `spawn`. A
    /// [`SystemSettingsSource::Fixed`] watch, a desktop without the portal and a build without the
    /// portal client never fire after their first answer. A portal that does not answer in a few
    /// seconds is the defaults, so this returns in a few seconds at worst; a caller that must not wait at all starts it beside its other work
    /// (`use_environment` does).
    pub async fn start(source: &SystemSettingsSource, spawn: &dyn Spawner) -> Self {
        let inner = match source {
            SystemSettingsSource::Fixed(prefs) => Inner::Fixed(*prefs),
            SystemSettingsSource::Portal => {
                Inner::Portal(settings_portal::Watch::start(spawn).await)
            }
        };
        SystemSettingsWatch { inner }
    }

    /// The preferences as last read.
    #[must_use]
    pub fn current(&self) -> SystemSettings {
        match &self.inner {
            Inner::Fixed(prefs) => *prefs,
            Inner::Portal(watch) => settings_from(watch.current()),
        }
    }

    /// Wait for the next change. `None` once the bus connection is gone, and at once for a fixed
    /// source.
    pub async fn changed(&mut self) -> Option<SystemSettings> {
        match &mut self.inner {
            Inner::Fixed(_) => None,
            Inner::Portal(watch) => watch.changed().await.map(settings_from),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{PortalPrefs, SystemSettingsSource, settings_from};
    use ui_style::appearance::system::{Contrast, SystemSettings};
    use ui_style::appearance::theme::Scheme;

    #[test]
    fn the_portal_values_map_to_the_toolkit_settings() {
        assert_eq!(
            settings_from(PortalPrefs::default()),
            SystemSettings::default()
        );
        let all = PortalPrefs {
            dark: true,
            reduced_motion: true,
            high_contrast: true,
        };
        assert_eq!(
            settings_from(all),
            SystemSettings::default()
                .with_scheme(Scheme::Dark)
                .with_reduced_motion(true)
                .with_contrast(Contrast::High)
        );
    }

    #[tokio::test]
    async fn a_fixed_source_answers_its_own_preferences_and_never_touches_the_bus() {
        let fixed = SystemSettings::default()
            .with_scheme(Scheme::Dark)
            .with_reduced_motion(true)
            .with_contrast(Contrast::High);
        assert_eq!(SystemSettingsSource::Fixed(fixed).read().await, fixed);
    }
}
