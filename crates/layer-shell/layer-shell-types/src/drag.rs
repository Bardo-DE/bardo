//! What a drag carries, what a target may do with it, and how it ended.

use std::fmt;

use serde::{Deserialize, Serialize};

/// What a drop target receives, offered under the mime types that describe it
/// (`drag::offer::offers`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DragPayload {
    /// Files or other resources, as URIs (`file:///tmp/shot.png`): offered as `text/uri-list`
    /// (CRLF-terminated lines, RFC 2483) and as plain text, one URI per line.
    Uris(Vec<String>),
    /// Text, offered under the same types a paste accepts (`clipboard::TEXT_MIME`).
    Text(String),
    /// Anything else, offered under exactly one mime type.
    Bytes { mime: Mime, bytes: Vec<u8> },
}

/// A mime type, `type/subtype` with optional parameters (`text/plain;charset=utf-8`).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Mime(String);

/// Why a string is not a mime type.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("not a mime type: {0:?}")]
pub struct BadMimeError(pub String);

impl Mime {
    /// `text` as a mime type: a non-empty type and subtype around one `/`, and no whitespace
    /// or control characters (a `wl_data_source.offer` string is matched whole by the target).
    ///
    /// # Errors
    ///
    /// `BadMimeError` when `text` is not a `type/subtype` pair, or holds whitespace or a control
    /// character.
    pub fn parse(text: &str) -> Result<Mime, BadMimeError> {
        let essence = text.split(';').next().unwrap_or_default();
        let well_formed = matches!(
            essence.split_once('/'),
            Some((kind, sub)) if !kind.is_empty() && !sub.is_empty() && !sub.contains('/')
        ) && !text.chars().any(|c| c.is_whitespace() || c.is_control());
        if well_formed {
            Ok(Mime(text.to_owned()))
        } else {
            Err(BadMimeError(text.to_owned()))
        }
    }

    /// The mime type as offered on the wire.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for Mime {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// What a drop target may do with the payload (`wl_data_source.set_actions`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DragActions {
    /// The source keeps its data.
    Copy,
    /// The target takes the data and the source deletes its own (when the outcome says so).
    Move,
    /// The target (or its user) chooses.
    CopyOrMove,
}

/// How a drag ended (`SurfaceHandle::start_drag`'s answer).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "v", rename_all = "snake_case")]
pub enum DragOutcome {
    /// A target took the payload and finished with it (`wl_data_source.dnd_finished`). On
    /// `Move` the source deletes its own copy.
    Dropped { action: DragAction },
    /// Released over nothing that accepted it, or the compositor ended the drag
    /// (`wl_data_source.cancelled`).
    Cancelled,
    /// The drag never started.
    Unsupported { why: DragError },
}

/// What the target did with a dropped payload.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DragAction {
    Copy,
    Move,
}

/// Why a drag did not start.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, thiserror::Error)]
#[serde(rename_all = "snake_case")]
pub enum DragError {
    /// No pointer button is held: a drag starts from a press that is still down.
    #[error("no pointer button is held")]
    NoPress,
    /// The held button was pressed on another surface.
    #[error("the held button was pressed on another surface")]
    PressedElsewhere,
    /// The compositor offers no `wl_data_device_manager`, or the seat has no data device yet.
    #[error("no data device")]
    NoDataDevice,
    /// The compositor's `wl_data_device_manager` is older than v3, so the source would never
    /// learn how the drop ended.
    #[error("wl_data_device_manager v3 is needed")]
    OldProtocol,
    /// Another drag from this client has not ended yet.
    #[error("a drag is already in progress")]
    InProgress,
    /// The surface has closed.
    #[error("the surface is gone")]
    NoSurface,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_mime_type_has_a_type_and_a_subtype() {
        const CASES: &[(&str, bool)] = &[
            ("text/uri-list", true),
            ("text/plain;charset=utf-8", true),
            ("application/x-shell-tile", true),
            ("UTF8_STRING", false),
            ("text/", false),
            ("/plain", false),
            ("text/plain/extra", false),
            ("text/plain; charset=utf-8", false),
            ("text/plain\n", false),
            ("", false),
        ];
        for &(text, valid) in CASES {
            assert_eq!(Mime::parse(text).is_ok(), valid, "{text:?}");
        }
        assert_eq!(
            Mime::parse("image/png").map(|mime| mime.to_string()),
            Ok("image/png".to_owned())
        );
    }

    #[test]
    fn outcomes_serialize_adjacently_tagged() {
        let cases = [
            (
                DragOutcome::Dropped {
                    action: DragAction::Copy,
                },
                r#"{"kind":"dropped","v":{"action":"copy"}}"#,
            ),
            (DragOutcome::Cancelled, r#"{"kind":"cancelled"}"#),
            (
                DragOutcome::Unsupported {
                    why: DragError::NoPress,
                },
                r#"{"kind":"unsupported","v":{"why":"no_press"}}"#,
            ),
        ];
        for (outcome, json) in cases {
            assert_eq!(serde_json::to_string(&outcome).ok().as_deref(), Some(json));
        }
    }
}
