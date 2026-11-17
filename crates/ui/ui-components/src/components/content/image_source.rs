//! Pictures the app supplies.
//!
//! Where the pixels come from ([`ImageSource`]) and how big they are ([`ImageSize`]). The UI kit never
//! fetches: a source is a `data:` URI or a `file:` URL, the two schemes a UI kit document's net
//! provider answers. `ProviderMark`'s favicon and the screenshot thumbnail both take one.

use std::path::Path;
use ui_core::error::DsError;
use ui_style::icon::url::IconUrl;

/// An image the app supplies, as a `data:` URI or a `file:` URL, written as an `<img>`'s `src`.
///
/// The caller decides which: a screenshot already on disk is cheaper as its path than re-encoded, a
/// favicon in memory is a `data:` URI. The field stays public so a caller holding a URI already
/// wraps it as it is.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ImageSource(pub String);

impl ImageSource {
    /// An absolute path as a percent-encoded `file://` URL. A relative path is refused: it has
    /// no meaning as a URL.
    ///
    /// # Errors
    /// [`DsError`] when `path` is relative.
    pub fn file(path: &Path) -> Result<Self, DsError> {
        IconUrl::file(path).map(|url| ImageSource(url.as_str().to_owned()))
    }

    /// A PNG's bytes as a `data:image/png;base64,…` URI.
    #[must_use]
    pub fn png(bytes: &[u8]) -> Self {
        ImageSource(IconUrl::png(bytes).as_str().to_owned())
    }
}

/// A picture's size in its own pixels. Only its ratio is read: a component draws the picture
/// at its own width and letterboxes it by this shape, so a caller never has to scale it first.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ImageSize {
    /// Pixels across.
    pub width: u32,
    /// Pixels down.
    pub height: u32,
}

#[cfg(test)]
mod tests {
    use super::ImageSource;
    use std::path::Path;

    #[test]
    fn a_file_is_an_absolute_file_url_and_bytes_are_a_png_uri() {
        let file = ImageSource::file(Path::new("/tmp/Screenshot 1.png")).expect("absolute");
        assert_eq!(file.0, "file:///tmp/Screenshot%201.png");
        ImageSource::file(Path::new("shot.png")).unwrap_err();
        assert_eq!(ImageSource::png(b"x").0, "data:image/png;base64,eA==");
    }
}
