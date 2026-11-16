//! Opening Settings at a place through its `org.freedesktop.Application` action `open-target`,
//! whose one parameter is the target's text (`extension:org.bardo.anyview.Heif`). It is the call
//! the launcher makes for a Settings result.

use crate::env::{BusRoute, Env};
use crate::error::PlatformError;
use crate::settings::{OpenSettings, Opening};
use std::collections::HashMap;
use ui_core::settings::SettingsTarget;
use zbus::zvariant::Value;

/// Settings' bus name, which is also its application id.
const SETTINGS: &str = "org.bardo.Settings";
/// Its object path: the id's dots as slashes.
const SETTINGS_PATH: &str = "/org/bardo/Settings";
const APPLICATION: &str = "org.freedesktop.Application";
/// Settings' action for a place, by the target's text.
const OPEN_TARGET: &str = "open-target";

/// [`OpenSettings`] over the session bus.
#[derive(Debug, Clone)]
pub struct SettingsApp {
    env: Env,
}

impl SettingsApp {
    /// Settings opened on `env`'s session bus.
    pub fn new(env: Env) -> Self {
        SettingsApp { env }
    }
}

impl OpenSettings for SettingsApp {
    fn open<'a>(&'a self, target: &'a SettingsTarget) -> Opening<'a> {
        Box::pin(async move {
            let connection = self
                .env
                .session_builder()?
                .build()
                .await
                .map_err(|error| PlatformError::bus("connect to the session bus", error))?;
            let platform_data: HashMap<&str, Value<'_>> = HashMap::new();
            connection
                .call_method(
                    Some(SETTINGS),
                    SETTINGS_PATH,
                    Some(APPLICATION),
                    "ActivateAction",
                    &(
                        OPEN_TARGET,
                        vec![Value::from(target.to_string())],
                        platform_data,
                    ),
                )
                .await
                .map(drop)
                .map_err(|error| PlatformError::bus("open Settings", error))
        })
    }

    fn present(&self) -> bool {
        !matches!(self.env.session, BusRoute::Absent)
    }
}
