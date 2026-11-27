//! Names and checked text on the inference wire. The tool and JSON ids are the model layer's
//! `model-types` ones, re-exported: one definition on both sides of the bridge.

use base64::Engine as _;
use base64::engine::general_purpose::STANDARD;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::fmt;

pub use model_types::{JsonText, OpaqueText, SchemaText, SignatureText, ToolCallId, ToolName};

/// Why wire text was refused.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum TextError {
    /// The text is not base64.
    #[error("not valid base64")]
    NotBase64,
    /// The text is not a version 00 W3C traceparent.
    #[error("not a W3C traceparent")]
    NotTraceparent,
}

/// Bytes that travel in JSON as base64 text (a screenshot, an audio frame). Debug shows the
/// length only.
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct Base64Bytes(pub Vec<u8>);

impl fmt::Debug for Base64Bytes {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Base64Bytes(<{} bytes>)", self.0.len())
    }
}

impl Serialize for Base64Bytes {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&STANDARD.encode(&self.0))
    }
}

impl<'de> Deserialize<'de> for Base64Bytes {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let text = String::deserialize(deserializer)?;
        STANDARD
            .decode(text)
            .map(Base64Bytes)
            .map_err(|e| serde::de::Error::custom(format!("{}: {e}", TextError::NotBase64)))
    }
}

/// The index of a file descriptor received with `SCM_RIGHTS` on the `Open` fd, in the order sent
/// with the frame that names it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct AttachIndex(pub u16);

/// A W3C trace context (`traceparent`): `00-<32 hex>-<16 hex>-<2 hex>`, lower case, neither id
/// all zero. Carried so one task is one trace across daemons; it holds ids only, never content.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct Traceparent(String);

impl Traceparent {
    /// The key it travels under in a D-Bus options dictionary.
    pub const KEY: &'static str = "traceparent";

    /// The header text, if it is a version 00 traceparent.
    ///
    /// # Errors
    ///
    /// [`TextError`] when `text` is not a trace header of the expected form.
    pub fn parse(text: &str) -> Result<Self, TextError> {
        let parts: Vec<&str> = text.split('-').collect();
        let hex = |s: &str, len: usize| {
            s.len() == len && s.bytes().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f'))
        };
        let nonzero = |s: &str| s.bytes().any(|b| b != b'0');
        let ok = matches!(parts.as_slice(),
            ["00", trace, span, flags]
                if hex(trace, 32) && nonzero(trace) && hex(span, 16) && nonzero(span) && hex(flags, 2));
        if ok {
            Ok(Self(text.to_owned()))
        } else {
            Err(TextError::NotTraceparent)
        }
    }

    /// The text.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<String> for Traceparent {
    type Error = TextError;
    fn try_from(text: String) -> Result<Self, TextError> {
        Self::parse(&text)
    }
}

impl From<Traceparent> for String {
    fn from(parent: Traceparent) -> String {
        parent.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn traceparents_follow_the_w3c_grammar() {
        let ok = "00-4bf92f3577b34da6a3ce929d0e0e4736-00f067aa0ba902b7-01";
        Traceparent::parse(ok).unwrap();
        let bad = [
            "",
            "01-4bf92f3577b34da6a3ce929d0e0e4736-00f067aa0ba902b7-01",
            "00-4BF92F3577B34DA6A3CE929D0E0E4736-00f067aa0ba902b7-01",
            "00-00000000000000000000000000000000-00f067aa0ba902b7-01",
            "00-4bf92f3577b34da6a3ce929d0e0e4736-0000000000000000-01",
            "00-4bf92f3577b34da6a3ce929d0e0e4736-00f067aa0ba902b7",
            "00-4bf92f3577b34da6a3ce929d0e0e473-00f067aa0ba902b7-01",
        ];
        for text in bad {
            assert!(Traceparent::parse(text).is_err(), "{text:?}");
        }
        assert_eq!(Traceparent::KEY, "traceparent");
    }
}
