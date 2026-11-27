//! The desktop's appearance preferences as plain values: dark or light, reduced motion, higher
//! contrast.
//!
//! On Linux they come from the settings portal:
//! `org.freedesktop.portal.Settings.ReadAll(["org.freedesktop.appearance"])` and
//! `SettingChanged`. The mappings are pure tables, tested against a fake `ReadAll` and
//! `SettingChanged` payload with no bus involved; only the client in `bus.rs` touches `zbus`, and
//! only on Linux with feature `desktop`. On macOS the same answers come from the user's defaults
//! (`macos.rs`). A desktop without the portal, or a build for a platform or feature set that has
//! neither (`absent.rs`), answers [`PortalSettings::default`].
//!
//! The crate knows no toolkit types: the program that draws maps [`PortalSettings`] to its own.
//! A watch runs as a task on the [`Spawner`] it is handed; no runtime is named here.

use base_spawn::Spawner;
use settings_store::latest::Receiver;

// The half that reaches `zbus`, the Mac's defaults, or a stand-in, with the same two entry points.
#[cfg_attr(all(target_os = "linux", feature = "desktop"), path = "bus.rs")]
#[cfg_attr(target_os = "macos", path = "macos.rs")]
#[cfg_attr(
    not(any(all(target_os = "linux", feature = "desktop"), target_os = "macos")),
    path = "absent.rs"
)]
mod bus;

/// What the desktop says about appearance. The default is what a desktop with no portal answers:
/// light, no motion preference, normal contrast.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct PortalSettings {
    /// The desktop prefers a dark colour scheme.
    pub dark: bool,
    /// The desktop asks for reduced motion.
    pub reduced_motion: bool,
    /// The desktop asks for higher contrast.
    pub high_contrast: bool,
}

/// `color-scheme`: 0 no preference, 1 prefer dark, 2 prefer light. No preference is light, so
/// this is whether the value asks for dark.
#[must_use]
pub fn dark_from_portal(value: u32) -> bool {
    value == 1
}

/// `reduced-motion`: 0 no preference, 1 reduce.
#[must_use]
pub fn reduced_motion_from_portal(value: u32) -> bool {
    value == 1
}

/// `contrast`: 0 no preference, 1 higher contrast.
#[must_use]
pub fn high_contrast_from_portal(value: u32) -> bool {
    value == 1
}

/// Every appearance preference, once. Never fails: a missing bus, portal or namespace is the
/// defaults.
pub async fn read() -> PortalSettings {
    bus::read_all().await.unwrap_or_default()
}

/// A subscription to the desktop's appearance preferences.
#[derive(Debug)]
pub struct Watch {
    changes: Receiver<PortalSettings>,
}

impl Watch {
    /// Subscribe and then read, the subscription's task running on `spawn`. A desktop without the
    /// portal and a build without the portal client never fire after their first answer. A portal
    /// that does not answer within [`Deadline::START`](base_time::deadline::Deadline::START) is the
    /// defaults, so this returns in a few seconds at worst; a caller that must not wait at all
    /// starts it beside its other work.
    pub async fn start(spawn: &dyn Spawner) -> Self {
        Watch {
            changes: bus::start(spawn).await,
        }
    }

    /// The preferences as last read.
    #[must_use]
    pub fn current(&self) -> PortalSettings {
        self.changes.latest()
    }

    /// Wait for the next change. `None` once the bus connection is gone.
    pub async fn changed(&mut self) -> Option<PortalSettings> {
        self.changes.changed().await
    }
}

#[cfg(test)]
mod tests {
    use super::{dark_from_portal, high_contrast_from_portal, reduced_motion_from_portal};

    #[test]
    fn the_scheme_table_matches_the_portal_spec() {
        const CASES: &[(u32, bool)] = &[(0, false), (1, true), (2, false), (99, false)];
        for &(value, want) in CASES {
            assert_eq!(dark_from_portal(value), want, "color-scheme {value}");
        }
    }

    #[test]
    fn the_reduced_motion_table_matches_the_portal_spec() {
        const CASES: &[(u32, bool)] = &[(0, false), (1, true), (7, false)];
        for &(value, want) in CASES {
            assert_eq!(
                reduced_motion_from_portal(value),
                want,
                "reduced-motion {value}"
            );
        }
    }

    #[test]
    fn the_contrast_table_matches_the_portal_spec() {
        const CASES: &[(u32, bool)] = &[(0, false), (1, true), (5, false)];
        for &(value, want) in CASES {
            assert_eq!(high_contrast_from_portal(value), want, "contrast {value}");
        }
    }
}
