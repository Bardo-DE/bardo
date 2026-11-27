//! Coordinate spaces as uninhabited markers.

use core::fmt::Debug;

use serde::{Deserialize, Serialize};

use crate::GridMax;

/// A coordinate space a point can live in. Implemented only by the three markers below, which
/// have no values: they exist to make `Point<WindowSpace>` and `Point<ImageSpace>` different
/// types.
pub trait CoordSpace: Copy + Eq + Debug + 'static {}

/// Logical pixels of the leased window's content, origin top-left, window-local.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WindowSpace {}

/// Pixels of the image the model was shown.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImageSpace {}

/// A normalised grid `0..=max` laid over the image the model was shown.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GridSpace {}

impl CoordSpace for WindowSpace {}
impl CoordSpace for ImageSpace {}
impl CoordSpace for GridSpace {}

/// Where a dialect's points live: the pixels of the image shown, or a normalised grid over it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(tag = "kind", content = "v", rename_all = "snake_case")]
pub enum ModelSpace {
    /// Pixels of the image the model was shown.
    Image,
    /// A normalised grid `0..=max` over that image.
    Grid(GridMax),
}
