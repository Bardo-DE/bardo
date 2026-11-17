//! Bardo arm's picture.

use std::collections::BTreeMap;

use super::app::BardoApp;
use super::shell::Shell;
use super::space::BardoSpace;
use super::window::BardoWindow;
use crate::keys::{AppKey, SpaceKey, ToplevelKey};
use crate::names::{AppId, OutputName, SpaceId};
use crate::toplevel::Announcement;
use desktop_protocols::vocab::ActorKind;

/// Which app is active and which window is key.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Active {
    /// The active app, if any.
    pub app: Option<AppKey>,
    /// The key window, if the active app has one.
    pub key: Option<ToplevelKey>,
    /// Who caused the last change.
    pub actor: ActorKind,
}

impl Default for Active {
    fn default() -> Self {
        Self {
            app: None,
            key: None,
            actor: ActorKind::Unknown,
        }
    }
}

/// Apps, windows, Spaces, the active pair and the shell's state.
///
/// Apps and Spaces are listed once announced (their first `done`); a window is reached by the
/// key of its toplevel.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Bardo {
    pub(crate) apps: BTreeMap<AppKey, BardoApp>,
    pub(crate) windows: BTreeMap<ToplevelKey, BardoWindow>,
    pub(crate) spaces: BTreeMap<SpaceKey, BardoSpace>,
    pub(crate) active: Active,
    pub(crate) shell: Shell,
}

impl Bardo {
    /// The announced apps, in creation order.
    pub fn apps(&self) -> impl Iterator<Item = &BardoApp> {
        self.apps
            .values()
            .filter(|a| a.announcement == Announcement::Announced)
    }

    /// One app, announced or not.
    #[must_use]
    pub fn app(&self, key: AppKey) -> Option<&BardoApp> {
        self.apps.get(&key)
    }

    /// The announced app with this key string.
    #[must_use]
    pub fn app_by_id(&self, id: &AppId) -> Option<&BardoApp> {
        self.apps().find(|a| a.id.as_ref() == Some(id))
    }

    /// The announced windows, in toplevel key order.
    pub fn windows(&self) -> impl Iterator<Item = &BardoWindow> {
        self.windows
            .values()
            .filter(|w| w.announcement == Announcement::Announced)
    }

    /// The window of a toplevel, announced or not.
    #[must_use]
    pub fn window(&self, key: ToplevelKey) -> Option<&BardoWindow> {
        self.windows.get(&key)
    }

    /// The announced windows of one app.
    pub fn windows_of(&self, app: AppKey) -> impl Iterator<Item = &BardoWindow> {
        self.windows().filter(move |w| w.app == Some(app))
    }

    /// The announced Spaces, in creation order.
    pub fn spaces(&self) -> impl Iterator<Item = &BardoSpace> {
        self.spaces
            .values()
            .filter(|s| s.announcement == Announcement::Announced)
    }

    /// One Space, announced or not.
    #[must_use]
    pub fn space(&self, key: SpaceKey) -> Option<&BardoSpace> {
        self.spaces.get(&key)
    }

    /// The announced Space with this id.
    #[must_use]
    pub fn space_by_id(&self, id: &SpaceId) -> Option<&BardoSpace> {
        self.spaces().find(|s| s.id.as_ref() == Some(id))
    }

    /// The announced Spaces of one output, left to right.
    #[must_use]
    pub fn spaces_on(&self, output: &OutputName) -> Vec<&BardoSpace> {
        let mut spaces: Vec<_> = self
            .spaces()
            .filter(|s| s.output.as_ref() == Some(output))
            .collect();
        spaces.sort_by_key(|s| s.position);
        spaces
    }

    /// The active app and key window.
    #[must_use]
    pub fn active(&self) -> &Active {
        &self.active
    }

    /// The shell's state.
    #[must_use]
    pub fn shell(&self) -> &Shell {
        &self.shell
    }

    /// Drops a window and, if it was key, the key.
    pub(crate) fn forget_window(&mut self, key: ToplevelKey) {
        self.windows.remove(&key);
        if self.active.key == Some(key) {
            self.active.key = None;
        }
        for window in self.windows.values_mut() {
            if window.parent == Some(key) {
                window.parent = None;
            }
        }
    }
}
