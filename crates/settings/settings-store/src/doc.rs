//! What a settings file is: a serde type that names its file and its format.

use crate::lenient::Limits;
use serde::Serialize;
use serde::de::DeserializeOwned;
use serde::ser::Error as _;

/// Why a value could not be written in a document's format.
#[derive(Debug, thiserror::Error)]
pub enum EncodeError {
    /// The value could not be written as TOML.
    #[error("encoding settings as TOML: {0}")]
    Toml(#[from] toml::ser::Error),
    /// The value could not be written as JSON (or as the one string of a CSS document).
    #[error("encoding settings as JSON: {0}")]
    Json(#[from] serde_json::Error),
}

/// A settings file's name inside its program's config directory: `appearance.toml`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FileName(pub &'static str);

/// How a settings file is written.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Format {
    /// TOML: every hand-edited settings file.
    Toml,
    /// JSON: machine-written stores such as `spaces.json`.
    Json,
    /// Raw CSS text (`style.css`): one string, never parsed at load, so no text is ever refused.
    /// A type stored this way is a newtype over `String`.
    Css,
}

impl Format {
    /// `value` as this format's text.
    ///
    /// # Errors
    ///
    /// Returns an [`EncodeError`] when the value cannot be written in the document's format.
    pub fn encode<T: Serialize>(self, value: &T) -> Result<String, EncodeError> {
        Ok(match self {
            Format::Toml => toml::to_string(value)?,
            Format::Json => serde_json::to_string_pretty(value)? + "\n",
            Format::Css => match serde_json::to_value(value)? {
                serde_json::Value::String(text) => text,
                _ => return Err(serde_json::Error::custom("a CSS document is one string").into()),
            },
        })
    }
}

/// A type stored as one settings file.
///
/// `settings_store_fs::Store` is the only way to load, save and watch it: a file is read leniently (a bad
/// value costs only its own key), written atomically, and a key the type does not read is reported
/// and never kept.
pub trait SettingsDoc:
    Serialize + DeserializeOwned + Default + Clone + PartialEq + 'static
{
    /// The file's name in the program's config directory.
    const FILE: FileName;
    /// How the file is written.
    const FORMAT: Format;

    /// The range of every bounded key of the file. A number outside its range is brought to the
    /// nearest end when the file is read, and reported in [`Loaded::clamped`](crate::Loaded). A
    /// document with a schema returns [`Limits::of_schema`]; the default is no ranges.
    #[must_use]
    fn limits() -> Limits {
        Limits::none()
    }
}
