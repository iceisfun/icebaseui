//! Geometric primitives used throughout BaseUI.
//!
//! All coordinates are in logical (DPI-independent) pixels with the origin in
//! the top-left corner and the y-axis pointing down, matching the convention
//! used by winit and most 2D UI toolkits.

/// A 2D vector / offset in logical pixels.
#[derive(Clone, Copy, PartialEq, Debug, Default)]
pub struct Vec2 {
    pub x: f32,
    pub y: f32,
}

impl Vec2 {
    pub const ZERO: Vec2 = Vec2 { x: 0.0, y: 0.0 };

    #[inline]
    pub const fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }

    #[inline]
    pub fn splat(v: f32) -> Self {
        Self { x: v, y: v }
    }

    #[inline]
    pub fn length(self) -> f32 {
        (self.x * self.x + self.y * self.y).sqrt()
    }
}

impl std::ops::Add for Vec2 {
    type Output = Vec2;
    #[inline]
    fn add(self, rhs: Vec2) -> Vec2 {
        Vec2::new(self.x + rhs.x, self.y + rhs.y)
    }
}

impl std::ops::Sub for Vec2 {
    type Output = Vec2;
    #[inline]
    fn sub(self, rhs: Vec2) -> Vec2 {
        Vec2::new(self.x - rhs.x, self.y - rhs.y)
    }
}

impl std::ops::Mul<f32> for Vec2 {
    type Output = Vec2;
    #[inline]
    fn mul(self, rhs: f32) -> Vec2 {
        Vec2::new(self.x * rhs, self.y * rhs)
    }
}

/// A point in logical pixel space.
#[derive(Clone, Copy, PartialEq, Debug, Default)]
pub struct Point {
    pub x: f32,
    pub y: f32,
}

impl Point {
    pub const ZERO: Point = Point { x: 0.0, y: 0.0 };

    #[inline]
    pub const fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }

    #[inline]
    pub fn to_vec2(self) -> Vec2 {
        Vec2::new(self.x, self.y)
    }
}

impl std::ops::Add<Vec2> for Point {
    type Output = Point;
    #[inline]
    fn add(self, rhs: Vec2) -> Point {
        Point::new(self.x + rhs.x, self.y + rhs.y)
    }
}

impl std::ops::Sub for Point {
    type Output = Vec2;
    #[inline]
    fn sub(self, rhs: Point) -> Vec2 {
        Vec2::new(self.x - rhs.x, self.y - rhs.y)
    }
}

/// A width/height pair in logical pixels. Never negative in normal use.
#[derive(Clone, Copy, PartialEq, Debug, Default)]
pub struct Size {
    pub width: f32,
    pub height: f32,
}

impl Size {
    pub const ZERO: Size = Size {
        width: 0.0,
        height: 0.0,
    };

    #[inline]
    pub const fn new(width: f32, height: f32) -> Self {
        Self { width, height }
    }

    #[inline]
    pub fn splat(v: f32) -> Self {
        Self::new(v, v)
    }

    #[inline]
    pub fn is_empty(self) -> bool {
        self.width <= 0.0 || self.height <= 0.0
    }
}

/// An axis-aligned rectangle described by its top-left origin and size.
#[derive(Clone, Copy, PartialEq, Debug, Default)]
pub struct Rect {
    pub origin: Point,
    pub size: Size,
}

impl Rect {
    pub const ZERO: Rect = Rect {
        origin: Point::ZERO,
        size: Size::ZERO,
    };

    #[inline]
    pub const fn new(origin: Point, size: Size) -> Self {
        Self { origin, size }
    }

    #[inline]
    pub fn from_xywh(x: f32, y: f32, width: f32, height: f32) -> Self {
        Self::new(Point::new(x, y), Size::new(width, height))
    }

    /// Construct a rect from its minimum (top-left) and maximum (bottom-right)
    /// corners. The corners are normalized so the result always has a
    /// non-negative size.
    #[inline]
    pub fn from_min_max(min: Point, max: Point) -> Self {
        let x = min.x.min(max.x);
        let y = min.y.min(max.y);
        Self::from_xywh(x, y, (max.x - min.x).abs(), (max.y - min.y).abs())
    }

    #[inline]
    pub fn min(self) -> Point {
        self.origin
    }

    #[inline]
    pub fn max(self) -> Point {
        Point::new(
            self.origin.x + self.size.width,
            self.origin.y + self.size.height,
        )
    }

    #[inline]
    pub fn left(self) -> f32 {
        self.origin.x
    }

    #[inline]
    pub fn top(self) -> f32 {
        self.origin.y
    }

    #[inline]
    pub fn right(self) -> f32 {
        self.origin.x + self.size.width
    }

    #[inline]
    pub fn bottom(self) -> f32 {
        self.origin.y + self.size.height
    }

    #[inline]
    pub fn width(self) -> f32 {
        self.size.width
    }

    #[inline]
    pub fn height(self) -> f32 {
        self.size.height
    }

    #[inline]
    pub fn center(self) -> Point {
        Point::new(
            self.origin.x + self.size.width * 0.5,
            self.origin.y + self.size.height * 0.5,
        )
    }

    /// Returns `true` if `p` lies within the rect (inclusive of the top-left
    /// edge, exclusive of the bottom-right edge).
    #[inline]
    pub fn contains(self, p: Point) -> bool {
        p.x >= self.left() && p.x < self.right() && p.y >= self.top() && p.y < self.bottom()
    }

    /// Shrinks the rect inward by `insets`. If the insets exceed the size the
    /// result is clamped to a zero-area rect at the shrunken origin.
    #[inline]
    pub fn shrink(self, insets: Insets) -> Rect {
        let x = self.left() + insets.left;
        let y = self.top() + insets.top;
        let w = (self.width() - insets.left - insets.right).max(0.0);
        let h = (self.height() - insets.top - insets.bottom).max(0.0);
        Rect::from_xywh(x, y, w, h)
    }

    /// Grows the rect outward by `insets`.
    #[inline]
    pub fn expand(self, insets: Insets) -> Rect {
        self.shrink(Insets {
            left: -insets.left,
            top: -insets.top,
            right: -insets.right,
            bottom: -insets.bottom,
        })
    }

    /// Returns the intersection of two rects, or a zero-area rect if disjoint.
    pub fn intersect(self, other: Rect) -> Rect {
        let min = Point::new(self.left().max(other.left()), self.top().max(other.top()));
        let max = Point::new(
            self.right().min(other.right()),
            self.bottom().min(other.bottom()),
        );
        if max.x <= min.x || max.y <= min.y {
            Rect::new(min, Size::ZERO)
        } else {
            Rect::from_min_max(min, max)
        }
    }
}

/// Per-edge spacing, used for padding and margins.
#[derive(Clone, Copy, PartialEq, Debug, Default)]
pub struct Insets {
    pub left: f32,
    pub top: f32,
    pub right: f32,
    pub bottom: f32,
}

impl Insets {
    pub const ZERO: Insets = Insets {
        left: 0.0,
        top: 0.0,
        right: 0.0,
        bottom: 0.0,
    };

    /// Equal spacing on all four edges.
    #[inline]
    pub const fn all(v: f32) -> Self {
        Self {
            left: v,
            top: v,
            right: v,
            bottom: v,
        }
    }

    /// Symmetric spacing: `horizontal` on left/right, `vertical` on top/bottom.
    #[inline]
    pub const fn symmetric(horizontal: f32, vertical: f32) -> Self {
        Self {
            left: horizontal,
            top: vertical,
            right: horizontal,
            bottom: vertical,
        }
    }

    #[inline]
    pub fn horizontal(self) -> f32 {
        self.left + self.right
    }

    #[inline]
    pub fn vertical(self) -> f32 {
        self.top + self.bottom
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rect_contains() {
        let r = Rect::from_xywh(10.0, 10.0, 100.0, 50.0);
        assert!(r.contains(Point::new(10.0, 10.0)));
        assert!(r.contains(Point::new(50.0, 30.0)));
        assert!(!r.contains(Point::new(110.0, 30.0)));
        assert!(!r.contains(Point::new(9.0, 30.0)));
    }

    #[test]
    fn rect_shrink_and_expand() {
        let r = Rect::from_xywh(0.0, 0.0, 100.0, 100.0);
        let s = r.shrink(Insets::all(10.0));
        assert_eq!(s, Rect::from_xywh(10.0, 10.0, 80.0, 80.0));
        assert_eq!(s.expand(Insets::all(10.0)), r);
    }

    #[test]
    fn rect_shrink_clamps() {
        let r = Rect::from_xywh(0.0, 0.0, 10.0, 10.0);
        let s = r.shrink(Insets::all(20.0));
        assert_eq!(s.size, Size::ZERO);
    }

    #[test]
    fn rect_intersect_disjoint() {
        let a = Rect::from_xywh(0.0, 0.0, 10.0, 10.0);
        let b = Rect::from_xywh(20.0, 20.0, 10.0, 10.0);
        assert!(a.intersect(b).size.is_empty());
    }
}
