//! [`DragValue`] — a Blender-style numeric field: click and drag horizontally
//! to scrub the value. Bound to a `Signal<f32>`.

use baseui_core::Signal;
use baseui_core::paint::{RectShape, Scene};
use baseui_core::{Point, Rect, Size};

use super::{EventCx, LayoutCx, PaintCx, Widget};
use crate::event::{InputEvent, PointerButton};
use crate::layout::Constraints;
use crate::text::FontId;

/// A draggable numeric value. Dragging right increases the value by
/// `speed` per logical pixel; the result is clamped to `[min, max]`.
///
/// ```no_run
/// use baseui::widget::DragValue;
/// use baseui::core::create_signal;
///
/// let fov = create_signal(60.0);
/// let _ = DragValue::new(fov).label("FOV").range(1.0, 179.0).speed(0.25);
/// ```
pub struct DragValue {
    value: Signal<f32>,
    label: Option<String>,
    speed: f32,
    min: f32,
    max: f32,
    decimals: usize,
    font_size: f32,
    hovered: bool,
    dragging: bool,
    /// Pointer x and value captured when the drag began.
    start_x: f32,
    start_val: f32,
}

impl DragValue {
    pub fn new(value: Signal<f32>) -> Self {
        DragValue {
            value,
            label: None,
            speed: 0.1,
            min: f32::NEG_INFINITY,
            max: f32::INFINITY,
            decimals: 2,
            font_size: 14.0,
            hovered: false,
            dragging: false,
            start_x: 0.0,
            start_val: 0.0,
        }
    }

    /// Prefix label shown before the value (e.g. axis name).
    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    /// Value change per logical pixel of horizontal drag.
    pub fn speed(mut self, speed: f32) -> Self {
        self.speed = speed;
        self
    }

    /// Clamp the value to `[min, max]`.
    pub fn range(mut self, min: f32, max: f32) -> Self {
        self.min = min;
        self.max = max;
        self
    }

    /// Number of fractional digits displayed.
    pub fn decimals(mut self, decimals: usize) -> Self {
        self.decimals = decimals;
        self
    }

    fn display(&self) -> String {
        let v = self.value.get();
        match &self.label {
            Some(l) => format!("{l}   {v:.*}", self.decimals),
            None => format!("{v:.*}", self.decimals),
        }
    }
}

impl Widget for DragValue {
    fn layout(&mut self, cx: &mut LayoutCx<'_>, constraints: Constraints) -> Size {
        // Size to a representative string so the field width is stable while the
        // value changes.
        let sample = match &self.label {
            Some(l) => format!("{l}   -0000.{}", "0".repeat(self.decimals)),
            None => format!("-0000.{}", "0".repeat(self.decimals)),
        };
        let text = cx.fonts.measure(&sample, self.font_size, FontId::Ui);
        let pad = cx.theme.spacing.md;
        let size = Size::new(text.width + pad * 2.0, text.height + cx.theme.spacing.sm * 2.0);
        constraints.constrain(size)
    }

    fn paint(&mut self, cx: &mut PaintCx<'_>, bounds: Rect, scene: &mut Scene) {
        let p = &cx.theme.palette;
        let bg = if self.dragging {
            p.active
        } else if self.hovered {
            p.hover
        } else {
            p.surface_variant
        };
        scene.push_rect(
            RectShape::fill(bounds, bg)
                .with_corner_radius(cx.theme.radius.sm)
                .with_border(1.0, p.border),
        );

        // Centered value text.
        let text = self.display();
        let ts = cx.fonts.measure(&text, self.font_size, FontId::Ui);
        let tx = bounds.left() + (bounds.width() - ts.width) * 0.5;
        let ty = bounds.top() + (bounds.height() - ts.height) * 0.5;
        scene.text(Point::new(tx, ty), text, self.font_size, p.text);
    }

    fn event(&mut self, _cx: &mut EventCx<'_>, bounds: Rect, event: &InputEvent) {
        match event {
            InputEvent::PointerMoved { pos } => {
                self.hovered = bounds.contains(*pos);
                if self.dragging {
                    let raw = self.start_val + (pos.x - self.start_x) * self.speed;
                    self.value.set(raw.clamp(self.min, self.max));
                }
            }
            InputEvent::PointerLeft => self.hovered = false,
            InputEvent::PointerPressed {
                pos,
                button: PointerButton::Primary,
            } => {
                if bounds.contains(*pos) {
                    self.dragging = true;
                    self.start_x = pos.x;
                    self.start_val = self.value.get();
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
