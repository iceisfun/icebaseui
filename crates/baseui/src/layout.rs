//! Layout constraints.
//!
//! BaseUI uses a single-pass, Flutter-style box model: a parent hands each child
//! a [`Constraints`] (a min/max size range), the child picks a concrete [`Size`]
//! within it and returns it, and the parent positions the child. Unbounded axes
//! use `f32::INFINITY` as the max.

use baseui_core::Size;

/// A min/max size range a widget must resolve to a concrete size within.
#[derive(Clone, Copy, Debug)]
pub struct Constraints {
    pub min: Size,
    pub max: Size,
}

impl Constraints {
    /// Exactly `size` (min == max).
    pub fn tight(size: Size) -> Self {
        Constraints {
            min: size,
            max: size,
        }
    }

    /// Anything from zero up to `max`.
    pub fn loose(max: Size) -> Self {
        Constraints {
            min: Size::ZERO,
            max,
        }
    }

    /// `max` width, unbounded height — the usual constraint for a child stacked
    /// in a column.
    pub fn loose_width(width: f32) -> Self {
        Constraints {
            min: Size::ZERO,
            max: Size::new(width, f32::INFINITY),
        }
    }

    /// `max` height, unbounded width — the usual constraint for a child in a row.
    pub fn loose_height(height: f32) -> Self {
        Constraints {
            min: Size::ZERO,
            max: Size::new(f32::INFINITY, height),
        }
    }

    /// Drop the minimum, keeping the maximum.
    pub fn loosen(self) -> Self {
        Constraints {
            min: Size::ZERO,
            max: self.max,
        }
    }

    /// Clamp `size` into this range.
    pub fn constrain(self, size: Size) -> Size {
        Size::new(
            clamp(size.width, self.min.width, self.max.width),
            clamp(size.height, self.min.height, self.max.height),
        )
    }
}

/// `f32::clamp` but tolerant of `max == INFINITY` and `min > max` (min wins).
fn clamp(v: f32, min: f32, max: f32) -> f32 {
    v.max(min).min(max.max(min))
}
