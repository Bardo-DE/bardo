//! The lines a table draws besides its rows: between its columns, and under its header.

use ui_core::geometry::units::Px;

/// The value of `data-scrolled`: `yes` when the content is `offset` past the top, so the rows sit
/// under the header, and `no` when the rows start at the top.
pub(crate) fn scrolled_word(offset: Px) -> &'static str {
    if offset.0 > 0.0 { "yes" } else { "no" }
}
