//! The sections a file's facts are listed in, and how much of them a summary line carries.

use ui_core::word::Word;

/// A section of the Info panel. Closed, so every pane names and orders the sections the same way.
///
/// The declaration order is the display order: the sections of what the file holds first, and
/// [`FactGroup::General`] (what any file has: its kind, size and dates) always last.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Word)]
pub enum FactGroup {
    /// An image's pixels: its size, colour, resolution and frames.
    Picture,
    /// The camera, lens and settings a photo was taken with.
    Camera,
    /// Where a photo was taken. Only the viewer's own Info panel lists it, never a preview pane.
    Location,
    /// What any recording has: its length, streams and codecs.
    Media,
    /// The sound of a recording: tags, sample rate and channels.
    Audio,
    /// The picture of a recording: size, frame rate and colour.
    Video,
    /// A document: its title, author, dates, producer and pages.
    Document,
    /// A text file: its encoding and lines.
    Text,
    /// A table: its rows and columns.
    Table,
    /// A structured document, such as JSON: what it holds at the top.
    Data,
    /// A book: its title, authors, publisher and chapters.
    Book,
    /// A font: its family, style and glyphs.
    Font,
    /// A folder: what it holds.
    Contents,
    /// An archive: its entries and sizes.
    Archive,
    /// What every file has, kind, size, dates and where it is. Always the last section.
    General,
}
