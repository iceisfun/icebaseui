//! [`Checkbox`] — a boolean toggle bound to a [`Signal`].

use baseui_core::Signal;
use baseui_core::paint::{RectShape, Scene};
use baseui_core::{Point, Rect, Size};

use super::{EventCx, LayoutCx, PaintCx, Widget};
use crate::event::{InputEvent, PointerButton};
use crate::layout::Constraints;
use crate::text::FontId;

const BOX: f32 = 18.0;

/// The box size, scaled by the global text scale.
fn box_size() -> f32 {
    BOX * crate::text::scale()
}

/// A labelled checkbox. Reads and writes a `Signal<bool>`, so toggling it
/// repaints everything bound to that signal.
///
/// ```no_run
/// use baseui::widget::Checkbox;
/// use baseui::core::create_signal;
///
/// let on = create_signal(true);
/// let _ = Checkbox::new(on, "Show ASCII");
/// ```
pub struct Checkbox {
    value: Signal<bool>,
    label: String,
    font_size: f32,
    hovered: bool,
    pressed: bool,
}

impl Checkbox {
    /// Binds to `value`: the box renders whatever the signal holds, and toggles
    /// it on click. The signal is the state, not a copy of it.
    pub fn new(value: Signal<bool>, label: impl Into<String>) -> Self {
        Checkbox {
            value,
            label: label.into(),
            font_size: 14.0,
            hovered: false,
            pressed: false,
        }
    }
}

impl Widget for Checkbox {
    fn layout(&mut self, cx: &mut LayoutCx<'_>, constraints: Constraints) -> Size {
        let text = cx.fonts.measure(&self.label, self.font_size, FontId::Ui);
        let gap = cx.theme.spacing.md;
        let b = box_size();
        let size = Size::new(b + gap + text.width, b.max(text.height));
        constraints.constrain(size)
    }

    fn paint(&mut self, cx: &mut PaintCx<'_>, bounds: Rect, scene: &mut Scene) {
        let p = &cx.theme.palette;
        let radius = cx.theme.radius.sm;
        let checked = self.value.get();

        let box_rect = Rect::from_xywh(
            bounds.left(),
            bounds.top() + (bounds.height() - box_size()) * 0.5,
            box_size(),
            box_size(),
        );

        // The box: accent-filled when checked, otherwise a bordered surface.
        let bg = if checked {
            p.accent
        } else if self.hovered {
            p.hover
        } else {
            p.surface_variant
        };
        let mut shape = RectShape::fill(box_rect, bg).with_corner_radius(radius);
        if !checked {
            shape = shape.with_border(1.0, p.border);
        }
        scene.push_rect(shape);

        // A simple check indicator: an inset mark drawn in the on-accent color.
        if checked {
            let inset = box_rect.shrink(baseui_core::Insets::all(5.0 * crate::text::scale()));
            scene.rounded_rect(inset, p.on_accent, radius * 0.5);
        }

        // Label.
        let text = cx.fonts.measure(&self.label, self.font_size, FontId::Ui);
        let ty = bounds.top() + (bounds.height() - text.height) * 0.5;
        let tx = box_rect.right() + cx.theme.spacing.md;
        scene.text(
            Point::new(tx, ty),
            self.label.clone(),
            self.font_size,
            p.text,
        );
    }

    fn event(&mut self, _cx: &mut EventCx<'_>, bounds: Rect, event: &InputEvent) {
        match event {
            InputEvent::PointerMoved { pos } => self.hovered = bounds.contains(*pos),
            InputEvent::PointerLeft => self.hovered = false,
            InputEvent::PointerPressed {
                pos,
                button: PointerButton::Primary,
            } => {
                if bounds.contains(*pos) {
                    self.pressed = true;
                }
            }
            InputEvent::PointerReleased {
                pos,
                button: PointerButton::Primary,
            } => {
                if self.pressed && bounds.contains(*pos) {
                    self.value.update(|v| *v = !*v);
                }
                self.pressed = false;
            }
            _ => {}
        }
    }
}
