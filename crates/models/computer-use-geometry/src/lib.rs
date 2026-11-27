//! Coordinates that carry the space they live in.
//!
//! A point of the leased window, a point of the image a model was shown and a point of a model's
//! normalised grid are different types, so mixing them does not compile. The crate holds only
//! this vocabulary: the actions that use it, the image preparation that maps between spaces and
//! the model capabilities that say where a model's points live each depend on it, and none of
//! them on another.
//!
//! ```
//! use computer_use_geometry::{Coord, ImageSpace, Point, Size};
//!
//! let at = Point::<ImageSpace>::new(Coord(10), Coord(20));
//! let size = Size::<ImageSpace>::new(Coord(640), Coord(480));
//! assert!(at.x < size.w);
//!
//! let json = serde_json::to_string(&at)?;
//! assert_eq!(serde_json::from_str::<Point<ImageSpace>>(&json)?, at);
//! # Ok::<(), serde_json::Error>(())
//! ```

mod geometry;
mod space;

pub use geometry::{Coord, DeviceSize, GridMax, Length, PixelFormat, Point, Rect, Scale120, Size};
pub use space::{CoordSpace, GridSpace, ImageSpace, ModelSpace, WindowSpace};
