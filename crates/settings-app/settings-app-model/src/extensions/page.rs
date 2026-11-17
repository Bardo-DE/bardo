//! The page's place among the pages: the UI kit declares it by the id `extensions`.

use settings_types::schema::Page;

/// The page's slug: what `--page` and `open-page` take.
pub const EXTENSIONS_SLUG: &str = "extensions";

/// The Extensions page as the catalogue holds it.
#[must_use]
pub fn extensions_page() -> Page {
    Page::declared(EXTENSIONS_SLUG)
}
