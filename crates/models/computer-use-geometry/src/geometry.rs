//! Points, sizes and the units around them.

use core::marker::PhantomData;

use serde::{Deserialize, Serialize};

use crate::CoordSpace;

/// One coordinate in a space's own unit. Whole numbers: a sub-pixel click means nothing, and
/// data stays `Eq`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Coord(pub u32);

/// A point in space `S`.
///
/// ```compile_fail
/// use computer_use_geometry::{Coord, ImageSpace, Point, WindowSpace};
/// let image: Point<ImageSpace> = Point::new(Coord(1), Coord(2));
/// let window: Point<WindowSpace> = image;
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(bound = "")]
pub struct Point<S: CoordSpace> {
    /// Distance from the left edge.
    pub x: Coord,
    /// Distance from the top edge.
    pub y: Coord,
    #[serde(skip)]
    space: PhantomData<S>,
}

impl<S: CoordSpace> Point<S> {
    /// The point `(x, y)` in space `S`.
    #[must_use]
    pub fn new(x: Coord, y: Coord) -> Self {
        Self {
            x,
            y,
            space: PhantomData,
        }
    }
}

/// A width and height in space `S`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(bound = "")]
pub struct Size<S: CoordSpace> {
    /// Width.
    pub w: Coord,
    /// Height.
    pub h: Coord,
    #[serde(skip)]
    space: PhantomData<S>,
}

impl<S: CoordSpace> Size<S> {
    /// A `w` by `h` size in space `S`.
    #[must_use]
    pub fn new(w: Coord, h: Coord) -> Self {
        Self {
            w,
            h,
            space: PhantomData,
        }
    }
}

/// A rectangle in space `S`: top-left corner and extent.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(bound = "")]
pub struct Rect<S: CoordSpace> {
    /// The top-left corner.
    pub origin: Point<S>,
    /// The width and height.
    pub size: Size<S>,
}

/// A distance along one axis in space `S` (a scroll distance).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(bound = "", transparent)]
pub struct Length<S: CoordSpace> {
    coord: Coord,
    space: PhantomData<S>,
}

impl<S: CoordSpace> Length<S> {
    /// A distance of `coord` along one axis.
    #[must_use]
    pub fn new(coord: Coord) -> Self {
        Self {
            coord,
            space: PhantomData,
        }
    }

    /// The distance as a bare coordinate.
    #[must_use]
    pub fn coord(&self) -> Coord {
        self.coord
    }
}

/// Device pixels per logical pixel, in 120ths (the `wp-fractional-scale` unit).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Scale120(pub u32);

/// The top of a model's normalised grid: 999 (Gemini) or 1000 (the Qwen3-VL family). A parameter
/// of the dialect profile, never a constant.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct GridMax(pub u16);

/// A buffer's size in device pixels (logical size times scale over 120).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct DeviceSize {
    /// Width in device pixels.
    pub w: u32,
    /// Height in device pixels.
    pub h: u32,
}

/// The `wl_shm` pixel formats a capturer hands over.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PixelFormat {
    /// Blue in the low byte, the top byte unused.
    Xrgb8888,
    /// Blue in the low byte, alpha in the top byte.
    Argb8888,
    /// Red in the low byte, the top byte unused.
    Xbgr8888,
    /// Red in the low byte, alpha in the top byte.
    Abgr8888,
}
