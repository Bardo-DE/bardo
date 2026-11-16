use crate::{PlateGrid, Template};

/// The plate grid of an export size (re-exported for callers that place things on it).
#[must_use]
pub fn grid_for(size: u32, t: &Template) -> PlateGrid {
    PlateGrid::for_canvas(size, t)
}
