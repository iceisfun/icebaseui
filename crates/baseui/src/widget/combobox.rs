//! [`ComboBox`] — a dropdown that selects one of a fixed list of options,
//! bound to a `Signal<usize>` (the selected index).
//!
//! Closed, it looks like a button showing the current option with a chevron.
//! Open, its list is drawn in the [`Scene`] overlay layer (like [`MenuBar`]),
//! and it swallows pointer events over its popup so clicks/hover don't leak to
//! widgets beneath.
//!
//! [`MenuBar`]: super::MenuBar

use baseui_core::paint::{RectShape, Scene};
use baseui_core::{Point, Rect, Signal, Size};

use super::{EventCx, LayoutCx, PaintCx, Widget};
use crate::event::{InputEvent, Key, PointerButton};
use crate::icon::glyphs;
use crate::layout::Constraints;
use crate::text::FontId;

/// A dropdown selection bound to a `Signal<usize>`.
pub struct ComboBox {
    options: Vec<String>,
    selected: Signal<usize>,
    open: bool,
    hovered: bool,
    hovered_item: Option<usize>,
    font_size: f32,
    width: Option<f32>,
}

impl ComboBox {
    pub fn new(selected: Signal<usize>, options: impl IntoIterator<Item = impl Into<String>>) -> Self {
        ComboBox {
            options: options.into_iter().map(Into::into).collect(),
            selected,
            open: false,
            hovered: false,
            hovered_item: None,
            font_size: 14.0,
            width: None,
        }
    }

    /// Fix the width (default: fill the available width).
    pub fn width(mut self, width: f32) -> Self {
        self.width = Some(width);
        self
    }

    fn selected_index(&self) -> usize {
        self.selected.get().min(self.options.len().saturating_sub(1))
    }

    /// Open/close the list. An open list is modal for the keyboard (it clears
    /// focus, so a focused text field stops receiving keystrokes behind it).
    fn set_open(&mut self, open: bool) {
        if self.open == open {
            return;
        }
        self.open = open;
        self.hovered_item = None;
        crate::popup::set_open(open);
    }

    fn item_height(&self, fonts: &crate::text::Fonts) -> f32 {
        fonts.line_height(self.font_size, FontId::Ui) + 8.0
    }

    /// (panel rect, per-item rects), all absolute. Empty if there are no options.
    fn dropdown(&self, fonts: &crate::text::Fonts, bounds: Rect) -> (Rect, Vec<Rect>) {
        let item_h = self.item_height(fonts);
        let panel = Rect::from_xywh(
            bounds.left(),
            bounds.bottom() + 2.0,
            bounds.width(),
            item_h * self.options.len() as f32,
        );
        let rects = (0..self.options.len())
            .map(|i| {
                Rect::from_xywh(
                    panel.left(),
                    panel.top() + i as f32 * item_h,
                    panel.width(),
                    item_h,
                )
            })
            .collect();
        (panel, rects)
    }
}

impl Widget for ComboBox {
    fn layout(&mut self, cx: &mut LayoutCx<'_>, constraints: Constraints) -> Size {
        let line_h = cx.fonts.line_height(self.font_size, FontId::Ui);
        let h = line_h + cx.theme.spacing.sm * 2.0 + 2.0;
        let w = self.width.unwrap_or_else(|| {
            if constraints.max.width.is_finite() {
                constraints.max.width
            } else {
                160.0
            }
        });
        constraints.constrain(Size::new(w, h))
    }

    fn paint(&mut self, cx: &mut PaintCx<'_>, bounds: Rect, scene: &mut Scene) {
        let p = &cx.theme.palette;
        let line_h = cx.fonts.line_height(self.font_size, FontId::Ui);

        // Closed button.
        let border = if self.open {
            p.accent
        } else if self.hovered {
            p.text_muted
        } else {
            p.border
        };
        scene.push_rect(
            RectShape::fill(bounds, p.surface_variant)
                .with_corner_radius(cx.theme.radius.sm)
                .with_border(1.0, border),
        );
        let ty = bounds.top() + (bounds.height() - line_h) * 0.5;
        if let Some(label) = self.options.get(self.selected_index()) {
            scene.text(
                Point::new(bounds.left() + cx.theme.spacing.md, ty),
                label.clone(),
                self.font_size,
                p.text,
            );
        }
        // Chevron.
        let chev = glyphs::CHEVRON_DOWN;
        let cw = cx.fonts.char_advance(chev.ch(), self.font_size, chev.font_id());
        scene.text_font(
            Point::new(bounds.right() - cx.theme.spacing.md - cw, ty),
            chev.ch().to_string(),
            self.font_size,
            p.text_muted,
            chev.font_id(),
        );

        // Open list, in the overlay layer.
        if self.open {
            let (panel, rects) = self.dropdown(cx.fonts, bounds);
            scene.begin_overlay();
            scene.push_rect(
                RectShape::fill(panel, p.surface)
                    .with_corner_radius(cx.theme.radius.md)
                    .with_border(1.0, p.border),
            );
            let selected = self.selected_index();
            for (i, r) in rects.iter().enumerate() {
                if self.hovered_item == Some(i) {
                    scene.rounded_rect(
                        r.shrink(baseui_core::Insets::symmetric(3.0, 1.0)),
                        p.hover,
                        cx.theme.radius.sm,
                    );
                } else if i == selected {
                    scene.rounded_rect(
                        r.shrink(baseui_core::Insets::symmetric(3.0, 1.0)),
                        p.selection,
                        cx.theme.radius.sm,
                    );
                }
                let iy = r.top() + (r.height() - line_h) * 0.5;
                scene.text(
                    Point::new(r.left() + cx.theme.spacing.md, iy),
                    self.options[i].clone(),
                    self.font_size,
                    p.text,
                );
            }
            scene.end_overlay();
        }
    }

    fn event(&mut self, cx: &mut EventCx<'_>, bounds: Rect, event: &InputEvent) {
        match event {
            InputEvent::PointerMoved { pos } => {
                self.hovered = bounds.contains(*pos);
                self.hovered_item = None;
                if self.open {
                    let (panel, rects) = self.dropdown(cx.fonts, bounds);
                    self.hovered_item = rects.iter().position(|r| r.contains(*pos));
                    if panel.contains(*pos) {
                        cx.consume();
                    }
                }
                if bounds.contains(*pos) {
                    cx.consume();
                }
            }
            InputEvent::PointerLeft => {
                self.hovered = false;
                self.hovered_item = None;
            }
            InputEvent::PointerPressed {
                pos,
                button: PointerButton::Primary,
            } => {
                if bounds.contains(*pos) {
                    let open = self.open;
                    self.set_open(!open);
                    cx.consume();
                    return;
                }
                if self.open {
                    let (panel, rects) = self.dropdown(cx.fonts, bounds);
                    if let Some(i) = rects.iter().position(|r| r.contains(*pos)) {
                        self.selected.set(i);
                        cx.consume();
                    } else if panel.contains(*pos) {
                        cx.consume();
                    }
                    self.set_open(false);
                }
            }
            // Escape closes the list (it is modal for the keyboard).
            InputEvent::Key {
                key: Key::Escape,
                pressed: true,
                ..
            } => {
                if self.open {
                    self.set_open(false);
                    cx.consume();
                }
            }
            _ => {}
        }
    }
}
