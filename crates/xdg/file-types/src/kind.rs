//! What a file is: its kind and the format inside the kind, each closed family of formats with
//! its extensions and MIME type, and the kind a MIME type names.

mod archive;
mod book;
mod delimiter;
mod family;
mod font;
mod media;
mod of_mime;
mod office;
mod raster;
mod tree;

pub use archive::ArchiveFormat;
pub use book::BookFormat;
pub use delimiter::Delimiter;
pub use family::{Family, from_extension, from_mime};
pub use font::FontFormat;
pub use media::MediaContainer;
pub use of_mime::of_mime;
pub use office::OfficeFormat;
pub use raster::RasterFormat;
pub use tree::TreeFormat;

use base_words::Word;

/// What a file is. A closed set, so every decision about files is a match on it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize, Word)]
#[serde(rename_all = "snake_case")]
#[word(case = snake)]
pub enum FormatKind {
    /// A PDF document.
    Pdf,
    /// A raster image.
    Raster,
    /// A vector image (SVG).
    Vector,
    /// A video.
    Video,
    /// An audio recording.
    Audio,
    /// A Markdown document.
    Markdown,
    /// Source code or configuration, highlighted.
    Code,
    /// Text with no more structure than lines.
    PlainText,
    /// A delimited table (CSV, TSV).
    Table,
    /// Structured data shown as a tree (JSON).
    Tree,
    /// A font file.
    Font,
    /// An archive whose entries are listed.
    Archive,
    /// A book or comic read page by page.
    Book,
    /// An office or iWork document, shown as facts and opened elsewhere.
    Office,
    /// A folder.
    Folder,
    /// A file of a type the viewer has no part for.
    Other,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kinds_are_stored_as_their_slugs() {
        for kind in FormatKind::ALL {
            assert_eq!(
                serde_json::to_value(kind).ok(),
                Some(serde_json::Value::from(kind.slug())),
                "the serde form of `{}` is not its slug",
                kind.slug()
            );
        }
        let json = serde_json::to_string(&FormatKind::PlainText).unwrap();
        assert_eq!(json, "\"plain_text\"");
        assert_eq!(
            serde_json::from_str::<FormatKind>(&json).unwrap(),
            FormatKind::PlainText
        );
    }
}
