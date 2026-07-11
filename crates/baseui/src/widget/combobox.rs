//! [`ComboBox`] — a dropdown that selects one of a fixed list of options,
//! bound to a `Signal<usize>` (the selected index).
//!
//! The list is a [`PopupMenu`], so it shares placement (flip/clamp to stay on
//! screen), overlay painting, event consumption, and keyboard modality with the
//! menu bar and context menus.

use baseui_core::paint::{RectShape, Scene};
use baseui_core::{Point, Rect, Signal, Size};

use super::{EventCx, LayoutCx, MenuItemSpec, PaintCx, PopupMenu, Widget};
use crate::event::{InputEvent, PointerButton};
use crate::icon::glyphs;
use crate::layout::Constraints;
use crate::text::FontId;

/// A dropdown selection bound to a `Signal<usize>`.
pub struct ComboBox {
    options: Vec<String>,
    selected: Signal<usize>,
    hovered: bool,
    font_size: f32,
    width: Option<f32>,
    menu: PopupMenu,
}

impl ComboBox {
    pub fn new(
        selected: Signal<usize>,
        options: impl IntoIterator<Item = impl Into<String>>,
    ) -> Self {
        ComboBox {
            options: options.into_iter().map(Into::into).collect(),
            selected,
            hovered: false,
            font_size: 14.0,
            width: None,
            menu: PopupMenu::new(),
        }
    }

    /// Fix the width (default: fill the available width).
    pub fn width(mut self, width: f32) -> Self {
        self.width = Some(width);
        self
    }

    fn selected_index(&self) -> usize {
        self.selected
            .get()
            .min(self.options.len().saturating_sub(1))
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

        let border = if self.menu.is_open() {
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

        let chev = glyphs::CHEVRON_DOWN;
        let cw = cx
            .fonts
            .char_advance(chev.ch(), self.font_size, chev.font_id());
        scene.text_font(
            Point::new(bounds.right() - cx.theme.spacing.md - cw, ty),
            chev.ch().to_string(),
            self.font_size,
            p.text_muted,
            chev.font_id(),
        );

        self.menu.paint(cx, scene);
    }

    fn event(&mut self, cx: &mut EventCx<'_>, bounds: Rect, event: &InputEvent) {
        // Clicking the button itself toggles the list; that click must not be
        // seen by the popup as a dismiss.
        if let InputEvent::PointerPressed {
            pos,
            button: PointerButton::Primary,
        } = event
        {
            if bounds.contains(*pos) {
                if self.menu.is_open() {
                    self.menu.close();
                } else {
                    let items = self
                        .options
                        .iter()
                        .map(|o| MenuItemSpec::new(o.clone()))
                        .collect();
                    self.menu.set_selected(Some(self.selected_index()));
                    self.menu.open_below(bounds, items);
                }
                cx.consume();
                return;
            }
        }

        if let Some(activation) = self.menu.event(cx, event) {
            self.selected.set(activation.index);
            return;
        }
        if cx.is_consumed() {
            return;
        }

        match event {
            InputEvent::PointerMoved { pos } => self.hovered = bounds.contains(*pos),
            InputEvent::PointerLeft => self.hovered = false,
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::text::Fonts;
    use crate::theme::Theme;
    use baseui_core::create_signal;

    /// Regression for the PopupMenu refactor: opening the list and clicking an
    /// option must set the bound signal.
    #[test]
    fn picking_an_option_sets_the_signal() {
        let Some(fonts) = Fonts::load() else {
            return;
        };
        let theme = Theme::dark();
        let screen = Size::new(800.0, 600.0);

        let selected = create_signal(0usize);
        let mut combo = ComboBox::new(selected, ["Alpha", "Beta", "Gamma"]);

        let mut lcx = LayoutCx {
            fonts: &fonts,
            theme: &theme,
            window: None,
        };
        let size = combo.layout(&mut lcx, Constraints::loose(Size::new(200.0, 40.0)));
        let bounds = Rect::new(Point::new(20.0, 40.0), size);

        // Click the button -> the list opens.
        let mut cx = EventCx::new(&fonts, &theme, screen);
        combo.event(
            &mut cx,
            bounds,
            &InputEvent::PointerPressed {
                pos: bounds.center(),
                button: PointerButton::Primary,
            },
        );
        assert!(combo.menu.is_open());

        // Lay the popup out, then click the second option.
        let mut cx = EventCx::new(&fonts, &theme, screen);
        combo.event(
            &mut cx,
            bounds,
            &InputEvent::PointerMoved {
                pos: bounds.center(),
            },
        );
        let panel = combo.menu.panel();
        let item_h = panel.height() / 3.0;
        let mut cx = EventCx::new(&fonts, &theme, screen);
        combo.event(
            &mut cx,
            bounds,
            &InputEvent::PointerPressed {
                pos: Point::new(panel.center().x, panel.top() + item_h * 1.5),
                button: PointerButton::Primary,
            },
        );

        assert_eq!(selected.get(), 1, "clicking 'Beta' selects index 1");
        assert!(!combo.menu.is_open());
    }
}
