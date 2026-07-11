//! [`Slider`] — drag a horizontal track to set a `Signal<f32>` within a range.

use baseui_core::Signal;
use baseui_core::paint::{RectShape, Scene};
use baseui_core::{Rect, Size};

use super::{EventCx, LayoutCx, PaintCx, Widget};
use crate::event::{InputEvent, PointerButton};
use crate::layout::Constraints;

const HEIGHT: f32 = 22.0;
const TRACK_H: f32 = 6.0;
const THUMB_R: f32 = 8.0;
const DEFAULT_WIDTH: f32 = 200.0;

/// A horizontal slider bound to a `Signal<f32>` over `[min, max]`.
pub struct Slider {
    value: Signal<f32>,
    min: f32,
    max: f32,
    width: f32,
    dragging: bool,
    hovered: bool,
}

impl Slider {
    pub fn new(value: Signal<f32>) -> Self {
        Slider {
            value,
            min: 0.0,
            max: 1.0,
            width: DEFAULT_WIDTH,
            dragging: false,
            hovered: false,
        }
    }

    /// Set the value range (default `0.0..=1.0`).
    pub fn range(mut self, min: f32, max: f32) -> Self {
        self.min = min;
        self.max = max;
        self
    }

    /// Set the track width in logical pixels.
    pub fn width(mut self, width: f32) -> Self {
        self.width = width;
        self
    }

    /// Normalized position of the current value in `0.0..=1.0`.
    fn fraction(&self) -> f32 {
        norm(self.value.get(), self.min, self.max)
    }

    fn apply_from_x(&self, x: f32, bounds: Rect) {
        let left = bounds.left() + THUMB_R;
        let track_w = (bounds.width() - THUMB_R * 2.0).max(1.0);
        let t = ((x - left) / track_w).clamp(0.0, 1.0);
        self.value.set(self.min + t * (self.max - self.min));
    }
}

impl Widget for Slider {
    fn layout(&mut self, _cx: &mut LayoutCx<'_>, constraints: Constraints) -> Size {
        let w = self.width.min(constraints.max.width);
        constraints.constrain(Size::new(w, HEIGHT))
    }

    fn paint(&mut self, cx: &mut PaintCx<'_>, bounds: Rect, scene: &mut Scene) {
        let p = &cx.theme.palette;
        let cy = bounds.top() + bounds.height() * 0.5;
        let left = bounds.left() + THUMB_R;
        let track_w = (bounds.width() - THUMB_R * 2.0).max(1.0);

        // Track.
        scene.rounded_rect(
            Rect::from_xywh(left, cy - TRACK_H * 0.5, track_w, TRACK_H),
            p.surface_variant,
            TRACK_H * 0.5,
        );
        // Filled portion.
        let frac = self.fraction();
        scene.rounded_rect(
            Rect::from_xywh(left, cy - TRACK_H * 0.5, track_w * frac, TRACK_H),
            p.accent,
            TRACK_H * 0.5,
        );
        // Thumb.
        let tx = left + track_w * frac;
        let thumb = Rect::from_xywh(tx - THUMB_R, cy - THUMB_R, THUMB_R * 2.0, THUMB_R * 2.0);
        let thumb_color = if self.dragging {
            p.accent
        } else if self.hovered {
            p.text
        } else {
            p.on_accent
        };
        scene.push_rect(
            RectShape::fill(thumb, thumb_color)
                .with_corner_radius(THUMB_R)
                .with_border(1.0, p.border),
        );
    }

    fn event(&mut self, _cx: &mut EventCx<'_>, bounds: Rect, event: &InputEvent) {
        match event {
            InputEvent::PointerMoved { pos } => {
                self.hovered = bounds.contains(*pos);
                if self.dragging {
                    self.apply_from_x(pos.x, bounds);
                }
            }
            InputEvent::PointerLeft => self.hovered = false,
            InputEvent::PointerPressed {
                pos,
                button: PointerButton::Primary,
            } => {
                if bounds.contains(*pos) {
                    self.dragging = true;
                    self.apply_from_x(pos.x, bounds);
                }
            }
            InputEvent::PointerReleased {
                button: PointerButton::Primary,
                ..
            } => self.dragging = false,
            _ => {}
        }
    }
}

/// Normalize `v` into `0.0..=1.0` over `[min, max]` (guards against min == max).
fn norm(v: f32, min: f32, max: f32) -> f32 {
    if (max - min).abs() < f32::EPSILON {
        0.0
    } else {
        ((v - min) / (max - min)).clamp(0.0, 1.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalization() {
        assert_eq!(norm(0.0, 0.0, 100.0), 0.0);
        assert_eq!(norm(50.0, 0.0, 100.0), 0.5);
        assert_eq!(norm(100.0, 0.0, 100.0), 1.0);
        assert_eq!(norm(150.0, 0.0, 100.0), 1.0); // clamped
        assert_eq!(norm(5.0, 5.0, 5.0), 0.0); // degenerate range
    }
}
