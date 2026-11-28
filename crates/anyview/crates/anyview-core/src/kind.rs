//! What a file is beyond its kind: the format detail, the syntax a highlighter knows, the text
//! encoding, and the media types of the kinds the viewer opens. The kinds and format families
//! themselves are `file-types`.

mod detail;
mod encoding;
mod opened;
mod syntax;

pub use detail::FormatDetail;
pub use encoding::TextEncoding;
pub use opened::{claimed_mimes, opened_globs, opened_mimes};
pub use syntax::SyntaxName;

pub(crate) use syntax::{for_extension as syntax_for_extension, for_name as syntax_for_name};
