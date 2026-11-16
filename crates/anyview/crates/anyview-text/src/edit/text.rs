//! A file's text as editing takes it up, and what stops a file from being edited.

use crate::encoding::{Coverage, TextCodec, detect};
use anyview_core::TextEncoding;
use std::io::{ErrorKind, Read as _};
use std::path::Path;

/// The largest file that is edited: the whole text is held in memory, and a larger file is only
/// read, so the editor never offers to open it.
pub const EDIT_BYTES: u64 = 8 * 1024 * 1024;

const UTF8_MARK: [u8; 3] = [0xEF, 0xBB, 0xBF];

/// The bytes a file begins with: the UTF-8 byte-order mark when `bom`, which saving puts back.
pub(crate) fn bom_mark(bom: bool) -> &'static [u8] {
    if bom { &UTF8_MARK } else { &[] }
}

/// Why a file is not edited.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum EditRefusal {
    /// The file is not UTF-8 text (it is UTF-16, or in a legacy encoding, or has bytes that are
    /// no text): saving it back would change more than the person typed.
    #[error("the file is not UTF-8 text")]
    NotUtf8,
    /// The file is larger than [`EDIT_BYTES`].
    #[error("the file is too large to edit")]
    TooLarge,
    /// The file could not be read.
    #[error("the file could not be read: {0}")]
    Unreadable(ErrorKind),
}

/// The text of a file that can be edited: valid UTF-8, with or without its byte-order mark.
/// Its line endings stay in the text, as the file has them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EditText {
    bytes: Vec<u8>,
    /// Whether the file began with a UTF-8 byte-order mark.
    bom: bool,
}

impl EditText {
    /// Read the file at `path` whole. Refuses a file that is not a regular file, is larger than
    /// [`EDIT_BYTES`], or is not UTF-8. Blocking.
    ///
    /// # Errors
    ///
    /// Returns [`EditRefusal::Unreadable`] when the file cannot be opened or read as a regular
    /// file, [`EditRefusal::TooLarge`] when it is over the edit limit, and [`EditRefusal::NotUtf8`]
    /// when it is not UTF-8 text.
    pub fn read(path: &Path) -> Result<EditText, EditRefusal> {
        let (file, meta) =
            anyview_fs::open_regular(path).map_err(|e| EditRefusal::Unreadable(e.kind()))?;
        if meta.len() > EDIT_BYTES {
            return Err(EditRefusal::TooLarge);
        }
        let mut bytes = Vec::with_capacity(usize::try_from(meta.len()).unwrap_or(0));
        // One byte more than allowed: a file that grew since it was measured is still refused.
        file.take(EDIT_BYTES + 1)
            .read_to_end(&mut bytes)
            .map_err(|e| EditRefusal::Unreadable(e.kind()))?;
        if bytes.len() as u64 > EDIT_BYTES {
            return Err(EditRefusal::TooLarge);
        }
        EditText::from_bytes(bytes)
    }

    /// The text of `bytes`, a whole file.
    ///
    /// # Errors
    ///
    /// Returns [`EditRefusal::NotUtf8`] when the bytes are not UTF-8 text (UTF-16, a legacy
    /// encoding, or bytes that are no text).
    pub fn from_bytes(mut bytes: Vec<u8>) -> Result<EditText, EditRefusal> {
        let detected = detect(&bytes, Coverage::Whole);
        if detected.codec != TextCodec::Unicode(TextEncoding::Utf8) {
            return Err(EditRefusal::NotUtf8);
        }
        let bom = detected.mark != 0;
        if bom {
            bytes = bytes.split_off(usize::from(detected.mark));
        }
        // The detector looked at the bytes before a mark too, but not past it.
        std::str::from_utf8(&bytes)
            .ok()
            .ok_or(EditRefusal::NotUtf8)?;
        Ok(EditText { bytes, bom })
    }

    pub(crate) fn into_parts(self) -> (Vec<u8>, bool) {
        (self.bytes, self.bom)
    }
}
