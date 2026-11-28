//! A file's type, as shared-mime-info names it.

use std::borrow::Cow;
use std::fmt;

use crate::error::MimeError;

/// A MIME type in lower case, `type/subtype`, with no parameters: `application/pdf`,
/// `inode/directory`. Never `text/plain; charset=utf-8`. Holding one proves the text is well formed.
#[derive(
    Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, serde::Serialize, serde::Deserialize,
)]
#[serde(try_from = "String", into = "String")]
pub struct MimeType(Cow<'static, str>);

/// Whether `byte` may stand in a type or subtype (lower case: callers lower-case first).
const fn is_token(byte: u8) -> bool {
    byte.is_ascii_lowercase()
        || byte.is_ascii_digit()
        || matches!(
            byte,
            b'!' | b'#' | b'$' | b'&' | b'^' | b'_' | b'.' | b'+' | b'-'
        )
}

/// Whether `bytes` is `type/subtype` of non-empty tokens with no upper case letter.
const fn is_lower_pair(bytes: &[u8]) -> bool {
    let mut slashes = 0;
    let mut part_len = 0;
    let mut at = 0;
    while at < bytes.len() {
        if bytes[at] == b'/' {
            if part_len == 0 {
                return false;
            }
            slashes += 1;
            part_len = 0;
        } else if is_token(bytes[at]) {
            part_len += 1;
        } else {
            return false;
        }
        at += 1;
    }
    slashes == 1 && part_len > 0
}

impl MimeType {
    /// The type of bytes that nothing else describes.
    pub const OCTET_STREAM: MimeType = MimeType(Cow::Borrowed("application/octet-stream"));

    /// A folder's type.
    pub const FOLDER: &'static str = "inode/directory";

    /// `text`, lower-cased, when it is one `type/subtype` pair of tokens (letters, digits and
    /// `! # $ & ^ _ . + -`).
    ///
    /// # Errors
    /// [`MimeError`] when `text` is not one `type/subtype` pair of tokens.
    pub fn new(text: &str) -> Result<MimeType, MimeError> {
        let lower = text.to_ascii_lowercase();
        if is_lower_pair(lower.as_bytes()) {
            Ok(MimeType(Cow::Owned(lower)))
        } else {
            Err(MimeError)
        }
    }

    /// A type written as a literal in a table, checked when the table is built (at compile time
    /// in a `const`).
    ///
    /// # Panics
    /// When `text` is not a lower-case `type/subtype` pair of tokens.
    #[must_use]
    pub const fn from_static(text: &'static str) -> MimeType {
        assert!(
            is_lower_pair(text.as_bytes()),
            "a MIME type literal is a lower-case type/subtype pair"
        );
        MimeType(Cow::Borrowed(text))
    }

    /// The type as text.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// The part before the slash: `image` of `image/png`.
    #[must_use]
    pub fn major(&self) -> &str {
        self.0.split('/').next().unwrap_or_default()
    }
}

impl fmt::Display for MimeType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl TryFrom<String> for MimeType {
    type Error = MimeError;

    fn try_from(text: String) -> Result<MimeType, MimeError> {
        MimeType::new(&text)
    }
}

impl From<MimeType> for String {
    fn from(mime: MimeType) -> String {
        mime.0.into_owned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn media_types_parse_to_their_lower_case_essence() {
        const CASES: &[(&str, &str, Option<&str>)] = &[
            ("plain", "text/plain", Some("text/plain")),
            ("upper case", "Image/PNG", Some("image/png")),
            (
                "suffix",
                "application/epub+zip",
                Some("application/epub+zip"),
            ),
            (
                "vendor",
                "application/vnd.ms-excel",
                Some("application/vnd.ms-excel"),
            ),
            ("parameter", "text/plain; charset=utf-8", None),
            ("no slash", "text", None),
            ("empty type", "/png", None),
            ("empty subtype", "image/", None),
            ("two slashes", "a/b/c", None),
            ("space", "text /plain", None),
            ("empty", "", None),
        ];
        for (name, text, want) in CASES {
            assert_eq!(
                MimeType::new(text).ok().as_ref().map(MimeType::as_str),
                *want,
                "{name}"
            );
        }
    }

    #[test]
    fn a_media_type_round_trips_and_a_bad_one_is_refused_on_load() {
        let mime = MimeType::new("image/webp").unwrap();
        let json = serde_json::to_string(&mime).unwrap();
        assert_eq!(json, "\"image/webp\"");
        assert_eq!(serde_json::from_str::<MimeType>(&json).unwrap(), mime);
        serde_json::from_str::<MimeType>("\"nonsense\"").unwrap_err();
        assert_eq!(
            MimeType::new("application/octet-stream"),
            Ok(MimeType::OCTET_STREAM)
        );
    }

    #[test]
    fn a_literal_names_its_major_part() {
        assert_eq!(MimeType::from_static("image/png").major(), "image");
    }
}
