//! [`Button`] — a clickable, themed push button.

use baseui_core::paint::{RectShape, Scene};
use baseui_core::{Point, Rect, Size};

use super::{EventCx, LayoutCx, PaintCx, Widget};
use crate::event::{InputEvent, PointerButton};
use crate::layout::Constraints;
use crate::text::FontId;

/// A push button. Its click handler typically mutates a [`Signal`], which — via
/// the reactive change hook — schedules a repaint automatically.
///
/// ```no_run
/// use baseui::widget::Button;
/// use baseui::core::create_signal;
///
/// let count = create_signal(0);
/// let button = Button::new("Increment").on_click(move || count.update(|c| *c += 1));
/// # let _ = button;
/// ```
///
/// [`Signal`]: baseui_core::Signal
pub struct Button {
    label: String,
    on_click: Box<dyn FnMut()>,
    /// Use the accent color as the background (a primary/call-to-action button).
    primary: bool,
    font_size: f32,
    hovered: bool,
    pressed: bool,
}

impl Button {
    /// A secondary button that does nothing. Attach an action with
    /// [`Button::on_click`].
    pub fn new(label: impl Into<String>) -> Self {
        Button {
            label: label.into(),
            on_click: Box::new(|| {}),
            primary: false,
            font_size: 14.0,
            hovered: false,
            pressed: false,
        }
    }

    /// Set the click handler, invoked on a primary-button press-and-release
    /// inside the button.
    pub fn on_click(mut self, f: impl FnMut() + 'static) -> Self {
        self.on_click = Box::new(f);
        self
    }

    /// Style as a primary (accent-colored) button.
    pub fn primary(mut self) -> Self {
        self.primary = true;
        self
    }

    fn padding(&self, theme: &crate::theme::Theme) -> (f32, f32) {
        (theme.spacing.lg, theme.spacing.sm + 2.0)
    }
}

impl Widget for Button {
    fn layout(&mut self, cx: &mut LayoutCx<'_>, constraints: Constraints) -> Size {
        let text = cx.fonts.measure(&self.label, self.font_size, FontId::Ui);
        let (px, py) = self.padding(cx.theme);
        let size = Size::new(text.width + px * 2.0, text.height + py * 2.0);
        constraints.constrain(size)
    }

    fn paint(&mut self, cx: &mut PaintCx<'_>, bounds: Rect, scene: &mut Scene) {
        let p = &cx.theme.palette;
        let radius = cx.theme.radius.md;

        // Background reflects interaction state.
        let base = if self.primary { p.accent } else { p.surface };
        let bg = if self.pressed {
            if self.primary {
                p.accent.lerp(p.active, 0.35)
            } else {
                p.active
            }
        } else if self.hovered {
            if self.primary {
                p.accent.lerp(p.on_accent, 0.12)
            } else {
                p.hover
            }
        } else {
            base
        };

        let mut shape = RectShape::fill(bounds, bg).with_corner_radius(radius);
        if !self.primary {
            shape = shape.with_border(1.0, p.border);
        }
        scene.push_rect(shape);

        // Centered label.
        let text_color = if self.primary { p.on_accent } else { p.text };
        let text_size = cx.fonts.measure(&self.label, self.font_size, FontId::Ui);
        let tx = bounds.left() + (bounds.width() - text_size.width) * 0.5;
        let ty = bounds.top() + (bounds.height() - text_size.height) * 0.5;
        scene.text(
            Point::new(tx, ty),
            self.label.clone(),
            self.font_size,
            text_color,
        );
    }

    fn event(&mut self, _cx: &mut EventCx<'_>, bounds: Rect, event: &InputEvent) {
        match event {
            InputEvent::PointerMoved { pos } => {
                self.hovered = bounds.contains(*pos);
            }
            InputEvent::PointerLeft => {
                self.hovered = false;
            }
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
                    (self.on_click)();
                }
                self.pressed = false;
            }
            _ => {}
        }
    }
}
