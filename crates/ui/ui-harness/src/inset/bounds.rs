//! A rectangle in the document's logical pixels, as edges.

/// A rectangle as its four edges.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Bounds {
    /// The left edge.
    pub left: f32,
    /// The top edge.
    pub top: f32,
    /// The right edge.
    pub right: f32,
    /// The bottom edge.
    pub bottom: f32,
}

impl Bounds {
    /// The rectangle at `x`, `y` of `width` by `height`.
    #[must_use]
    pub fn at(x: f32, y: f32, width: f32, height: f32) -> Self {
        Bounds {
            left: x,
            top: y,
            right: x + width,
            bottom: y + height,
        }
    }

    /// The width.
    #[must_use]
    pub fn width(&self) -> f32 {
        self.right - self.left
    }

    /// The height.
    #[must_use]
    pub fn height(&self) -> f32 {
        self.bottom - self.top
    }

    /// The vertical middle.
    #[must_use]
    pub fn middle(&self) -> f32 {
        f32::midpoint(self.top, self.bottom)
    }

    /// Whether the vertical middle of `other` lies between this rectangle's top and bottom: it
    /// is on a line this box spans.
    #[must_use]
    pub fn spans_middle_of(&self, other: &Bounds) -> bool {
        (self.top..=self.bottom).contains(&other.middle())
    }

    /// The smallest rectangle holding both.
    #[must_use]
    pub fn union(&self, other: &Bounds) -> Bounds {
        Bounds {
            left: self.left.min(other.left),
            top: self.top.min(other.top),
            right: self.right.max(other.right),
            bottom: self.bottom.max(other.bottom),
        }
    }
}
