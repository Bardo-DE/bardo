//! What a file is called and what it is.
//!
//! [`FileName`] is one path component as Linux allows it and [`MimeType`] is a media type, parsed.
//! [`FormatKind`] is the closed set of kinds of file a viewer tells apart, and the format families
//! ([`RasterFormat`], [`ArchiveFormat`], ...) each state their own extensions and MIME type, so
//! there is one table from a MIME type to a kind ([`of_mime`]) and one from an extension.
//!
//! Nothing here reads a file or the environment.

mod error;
mod file_name;
mod kind;
mod mime_type;

pub use error::{MimeError, NameError};
pub use file_name::FileName;
pub use kind::{
    ArchiveFormat, BookFormat, Delimiter, Family, FontFormat, FormatKind, MediaContainer,
    OfficeFormat, RasterFormat, TreeFormat, from_extension, from_mime, of_mime,
};
pub use mime_type::MimeType;
