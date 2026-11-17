//! `casement --write-schema DIR`: the schema detent and the Settings app read.

use super::ConfigError;
use super::sections::{
    GesturesSection, GpuSection, KeyboardSection, NightLightSection, OutputsSection,
    PointerSection, SpacesSection, TouchpadSection, WindowsSection, XwaylandSection,
};
use settings_types::schema::{AppId, FilePath, ForeignTables, Schema, SettingsSchema};
use std::path::Path;

/// The schema's version: bumped when a key's meaning changes, never for a new key.
const VERSION: u16 = 1;

/// Every key the document reads, as `<app-id>.settings.toml` ships it. `[bindings]` is not in
/// it: a chord table has no fixed keys to draw.
#[must_use]
pub fn schema() -> Schema {
    Schema {
        app: AppId("compositor".to_owned()),
        file: FilePath("compositor/settings.toml".to_owned()),
        version: VERSION,
        foreign: ForeignTables::default(),
        key: [
            WindowsSection::schema(),
            SpacesSection::schema(),
            KeyboardSection::schema(),
            OutputsSection::schema(),
            PointerSection::schema(),
            TouchpadSection::schema(),
            GesturesSection::schema(),
            GpuSection::schema(),
            NightLightSection::schema(),
            XwaylandSection::schema(),
        ]
        .into_iter()
        .flat_map(|schema| schema.key)
        .collect(),
        pages: Vec::new(),
        footnotes: Vec::new(),
    }
}

/// Writes `compositor.settings.toml` into `to` (created if needed, replaced atomically).
///
/// # Errors
///
/// Returns [`ConfigError`] when the schema cannot be written to `to`.
pub fn write_schema(to: &Path) -> Result<(), ConfigError> {
    settings_store_fs::write_schema(&schema(), to)?;
    Ok(())
}
