//! The document.

use super::ConfigError;
use super::sections::{
    BindingsSection, GesturesSection, GpuSection, KeyboardSection, NightLightSection,
    OutputsSection, PointerSection, SpacesSection, TouchpadSection, WindowsSection,
    XwaylandSection,
};
use serde::{Deserialize, Serialize};
use settings_store::{FileName, Format, SettingsDoc};
use settings_store_fs::{ConfigRoot, Store, Watch};
use settings_types::AppName;
use std::path::Path;

/// `compositor/settings.toml`: one section per `CompositorSettings` part. A key it does not read
/// is reported on load and dropped on save (ui-settings).
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct CompositorDoc {
    /// `windows.*`.
    pub windows: WindowsSection,
    /// `spaces.*`.
    pub spaces: SpacesSection,
    /// `keyboard.*`.
    pub keyboard: KeyboardSection,
    /// `[bindings]` (C30: the compositor's document carries the defaults).
    pub bindings: BindingsSection,
    /// `outputs.*`.
    pub outputs: OutputsSection,
    /// `pointer.*` (mice and trackballs).
    pub pointer: PointerSection,
    /// `touchpad.*`.
    pub touchpad: TouchpadSection,
    /// `gestures.*`.
    pub gestures: GesturesSection,
    /// `gpu.*`.
    pub gpu: GpuSection,
    /// `night_light.*`.
    pub night_light: NightLightSection,
    /// `xwayland.*` (older X11 apps; read at start).
    pub xwayland: XwaylandSection,
}

impl SettingsDoc for CompositorDoc {
    const FILE: FileName = FileName("settings.toml");
    const FORMAT: Format = Format::Toml;
}

/// The program's directory name under the config home.
const APP: AppName = AppName::desktop("compositor");

/// The document under `home` (a missing file: the defaults). A key the file sets that the
/// document does not read, or whose value it refuses, is reported in the log and costs only
/// itself.
///
/// `Some(dir)` stands in for `$XDG_CONFIG_HOME`, so the file is `dir/compositor/settings.toml`
/// (ui-settings' scratch root); `None` reads the environment.
///
/// # Errors
///
/// Returns [`ConfigError`] when the settings file exists but cannot be read.
pub fn load(home: Option<&Path>) -> Result<CompositorDoc, ConfigError> {
    let loaded = store(home).load::<CompositorDoc>();
    for key in &loaded.unknown {
        tracing::warn!("settings.toml: {key}");
    }
    for key in &loaded.invalid {
        tracing::warn!("settings.toml: {key}");
    }
    Ok(loaded.value)
}

/// The `xwayland.*` keys of `home`'s document, read without the log lines `load` writes (the
/// settings are loaded, and reported, once more when the compositor starts).
#[must_use]
pub fn xwayland_of(home: Option<&Path>) -> XwaylandSection {
    store(home).load::<CompositorDoc>().value.xwayland
}

/// `home` as the ui-settings store it names.
pub(crate) fn store(home: Option<&Path>) -> Store {
    let root = home.map_or(ConfigRoot::Xdg, |dir| {
        ConfigRoot::Scratch(dir.to_path_buf())
    });
    Store::new(root, APP)
}

/// A watch on `home`'s document: every settled change of the file arrives whole (ui-settings,
/// debounced). It runs as a task on `spawn`.
pub(crate) fn watch(home: Option<&Path>, spawn: &dyn base_spawn::Spawner) -> Watch<CompositorDoc> {
    store(home).watch::<CompositorDoc>(spawn)
}
