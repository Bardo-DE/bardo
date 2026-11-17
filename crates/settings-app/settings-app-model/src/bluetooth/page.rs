//! The page's place among the pages: the UI kit declares it by the id `bluetooth`.

use settings_types::schema::Page;

/// The page's slug: what `--page` and `open-page` take.
pub const BLUETOOTH_SLUG: &str = "bluetooth";

/// The Bluetooth page as the catalogue holds it.
#[must_use]
pub fn bluetooth_page() -> Page {
    Page::declared(BLUETOOTH_SLUG)
}
