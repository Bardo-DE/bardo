//! `appearance.toml` as a settings document.

use super::{AppearanceFile, bardo_schema};
use settings_store::{FileName, Format, Limits, SettingsDoc};

impl SettingsDoc for AppearanceFile {
    const FILE: FileName = FileName("appearance.toml");
    const FORMAT: Format = Format::Toml;

    fn limits() -> Limits {
        Limits::of_schema(&bardo_schema())
    }
}
