//! Where in Settings a result or a button points.
//!
//! Where in Settings a result or a button points: `ui_core::settings::SettingsTarget`
//! (`page:wallpaper`, `key:dock.magnified_size_px`, `account:<id>`), the one type every entrance
//! hands over. It is pure, so the pure crates hold it directly.
//!
//! Its serde form is its text. `ui-core` gives the type `Display` and `FromStr` but no serde, so
//! a field that crosses a wire says `#[serde(with = "shell_launcher::settings_target::text")]`.

pub use ui_core::settings::{AccountRef, KeyPath, PageId, SettingsTarget};

/// serde `with` module: a target as its text.
pub mod text {
    use serde::{Deserialize, Deserializer, Serializer};
    use ui_core::settings::SettingsTarget;

    /// Write `target` as its text.
    ///
    /// # Errors
    ///
    /// Returns the serializer's error when `out` cannot take the text.
    pub fn serialize<S: Serializer>(target: &SettingsTarget, out: S) -> Result<S::Ok, S::Error> {
        out.collect_str(target)
    }

    /// Read a target from its text.
    ///
    /// # Errors
    ///
    /// Returns the deserializer's error when the text is not a target.
    pub fn deserialize<'de, D: Deserializer<'de>>(input: D) -> Result<SettingsTarget, D::Error> {
        let text = String::deserialize(input)?;
        text.parse().map_err(serde::de::Error::custom)
    }
}

/// The page `id` (`wifi`, `bluetooth`, `wallpaper`) as a target.
#[must_use]
pub fn page(id: &str) -> SettingsTarget {
    SettingsTarget::Page(PageId::new(id))
}
