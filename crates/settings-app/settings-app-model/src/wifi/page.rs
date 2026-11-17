//! The page's place among the pages: the UI kit declares it by the id `wifi`.

use settings_types::schema::Page;

/// The page's slug: what `--page` and `open-page` take.
pub const WIFI_SLUG: &str = "wifi";

/// The Wi-Fi page as the catalogue holds it.
#[must_use]
pub fn wifi_page() -> Page {
    Page::declared(WIFI_SLUG)
}
