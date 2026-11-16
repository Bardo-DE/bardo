//! A fake Settings.

use super::locked;
use crate::error::PlatformError;
use crate::settings::{OpenSettings, Opening};
use std::sync::{Arc, Mutex};
use ui_core::settings::SettingsTarget;

/// An [`OpenSettings`] that records the places it was asked to open.
#[derive(Debug, Clone, Default)]
pub struct FakeSettings {
    opened: Arc<Mutex<Vec<SettingsTarget>>>,
}

impl FakeSettings {
    /// Every place opened, oldest first.
    pub fn opened(&self) -> Vec<SettingsTarget> {
        locked(&self.opened).clone()
    }
}

impl OpenSettings for FakeSettings {
    fn open<'a>(&'a self, target: &'a SettingsTarget) -> Opening<'a> {
        Box::pin(async move {
            locked(&self.opened).push(target.clone());
            Ok::<(), PlatformError>(())
        })
    }
}
