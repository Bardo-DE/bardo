//! Where in Settings a result or a button points: `ui_core::settings::SettingsTarget`
//! (`page:wallpaper`, `key:dock.magnified_size_px`, `account:<id>`), the one type every entrance
//! hands over. It is pure, so the pure crates hold it directly.
//!
//! Its serde form is its text. `ds-core` gives the type `Display` and `FromStr` but no serde, so
//! a field that crosses a wire says `#[serde(with = "shell_launcher::settings_target::text")]`.

pub use ui_core::settings::{AccountRef, KeyPath, PageId, SettingsTarget};

/// serde `with` module: a target as its text.
pub mod text {
    use ui_core::settings::SettingsTarget;
    use serde::{Deserialize, Deserializer, Serializer};

    /// Write `target` as its text.
    pub fn serialize<S: Serializer>(target: &SettingsTarget, out: S) -> Result<S::Ok, S::Error> {
        out.collect_str(target)
    }

    /// Read a target from its text.
    pub fn deserialize<'de, D: Deserializer<'de>>(input: D) -> Result<SettingsTarget, D::Error> {
        let text = String::deserialize(input)?;
        text.parse().map_err(serde::de::Error::custom)
    }
}

/// The page `id` (`wifi`, `bluetooth`, `wallpaper`) as a target.
pub fn page(id: &str) -> SettingsTarget {
    SettingsTarget::Page(PageId::new(id))
}
