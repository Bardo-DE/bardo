//! Logical pixels, and the points, sizes and rects built from them.
//!
//! `f32`, like the renderer's own layout: a rect from `onmounted` is fractional. That makes these
//! `PartialEq` without `Eq`, the one deliberate exception to the rule that data is `Eq`
//! (ARCHITECTURE.md "Repo rules").

use std::ops::{Add, Sub};

/// A length in logical pixels.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Default)]
pub struct Px(pub f32);

impl Add for Px {
    type Output = Px;
    fn add(self, rhs: Px) -> Px {
        Px(self.0 + rhs.0)
    }
}

impl Sub for Px {
    type Output = Px;
    fn sub(self, rhs: Px) -> Px {
        Px(self.0 - rhs.0)
    }
}

/// A point in a surface's coordinates.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Point {
    /// Distance from the left edge.
    pub x: Px,
    /// Distance from the top edge.
    pub y: Px,
}

/// A width and a height.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Size {
    /// Horizontal extent.
    pub width: Px,
    /// Vertical extent.
    pub height: Px,
}

/// An axis-aligned rectangle.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Rect {
    /// The top-left corner.
    pub origin: Point,
    /// Its extent.
    pub size: Size,
}

impl Rect {
    /// The left edge.
    #[must_use]
    pub fn left(self) -> Px {
        self.origin.x
    }

    /// The top edge.
    #[must_use]
    pub fn top(self) -> Px {
        self.origin.y
    }

    /// The right edge.
    #[must_use]
    pub fn right(self) -> Px {
        self.origin.x + self.size.width
    }

    /// The bottom edge.
    #[must_use]
    pub fn bottom(self) -> Px {
        self.origin.y + self.size.height
    }
}
