//! [`Toolbar`] — a horizontal strip of icon/text buttons, toggles, separators,
//! and flexible spacers.
//!
//! Buttons run a command on click; toggles flip a `Signal<bool>` and show an
//! active background; a spacer pushes everything after it to the right.

use baseui_core::Signal;
use baseui_core::paint::Scene;
use baseui_core::{Point, Rect, Size};

use super::{EventCx, LayoutCx, PaintCx, Widget};
use crate::event::{InputEvent, PointerButton};
use crate::icon::Icon;
use crate::layout::Constraints;
use crate::text::FontId;

type Action = Box<dyn FnMut()>;

enum Item {
    Button {
        icon: Option<Icon>,
        label: Option<String>,
        on: Action,
        rect: Rect,
    },
    Toggle {
        icon: Option<Icon>,
        label: Option<String>,
        value: Signal<bool>,
        rect: Rect,
    },
    Separator {
        rect: Rect,
    },
    Spacer,
}

/// A horizontal toolbar.
pub struct Toolbar {
    items: Vec<Item>,
    hovered: Option<usize>,
    pressed: Option<usize>,
    font_size: f32,
    bar_h: f32,
}

impl Toolbar {
    pub fn new() -> Self {
        Toolbar {
            items: Vec::new(),
            hovered: None,
            pressed: None,
            font_size: 13.0,
            bar_h: 36.0,
        }
    }

    /// An icon-only button.
    pub fn button_icon(self, icon: Icon, on: impl FnMut() + 'static) -> Self {
        self.push_button(Some(icon), None, on)
    }

    /// A text button.
    pub fn button(self, label: impl Into<String>, on: impl FnMut() + 'static) -> Self {
        self.push_button(None, Some(label.into()), on)
    }

    /// An icon + text button.
    pub fn button_labeled(
        self,
        icon: Icon,
        label: impl Into<String>,
        on: impl FnMut() + 'static,
    ) -> Self {
        self.push_button(Some(icon), Some(label.into()), on)
    }

    fn push_button(
        mut self,
        icon: Option<Icon>,
        label: Option<String>,
        on: impl FnMut() + 'static,
    ) -> Self {
        self.items.push(Item::Button {
            icon,
            label,
            on: Box::new(on),
            rect: Rect::ZERO,
        });
        self
    }

    /// An icon toggle bound to a `Signal<bool>`.
    pub fn toggle_icon(mut self, icon: Icon, value: Signal<bool>) -> Self {
        self.items.push(Item::Toggle {
            icon: Some(icon),
            label: None,
            value,
            rect: Rect::ZERO,
        });
        self
    }

    /// A separator line.
    pub fn separator(mut self) -> Self {
        self.items.push(Item::Separator { rect: Rect::ZERO });
        self
    }

    /// A flexible spacer that pushes following items to the right.
    pub fn spacer(mut self) -> Self {
        self.items.push(Item::Spacer);
        self
    }

    fn content_width(&self, fonts: &crate::text::Fonts, item: &Item, pad: f32) -> f32 {
        let content = |icon: &Option<Icon>, label: &Option<String>| -> f32 {
            let mut w = 0.0;
            if let Some(icon) = icon {
                w += fonts.char_advance(icon.ch(), self.font_size, icon.font_id());
            }
            if let Some(label) = label {
                if icon.is_some() {
                    w += pad * 0.5;
                }
                w += fonts.measure(label, self.font_size, FontId::Ui).width;
            }
            w + pad * 2.0
        };
        match item {
            Item::Button { icon, label, .. } => content(icon, label).max(self.bar_h - 8.0),
            Item::Toggle { icon, label, .. } => content(icon, label).max(self.bar_h - 8.0),
            Item::Separator { .. } => 9.0,
            Item::Spacer => 0.0,
        }
    }
}

impl Default for Toolbar {
    fn default() -> Self {
        Toolbar::new()
    }
}

impl Widget for Toolbar {
    fn layout(&mut self, cx: &mut LayoutCx<'_>, constraints: Constraints) -> Size {
        let line_h = cx.fonts.line_height(self.font_size, FontId::Ui);
        self.bar_h = line_h + cx.theme.spacing.md * 2.0;
        let pad = cx.theme.spacing.md;
        let w = if constraints.max.width.is_finite() {
            constraints.max.width
        } else {
            600.0
        };

        // Fixed widths + spacer distribution.
        let mut fixed = 0.0;
        let mut spacers = 0;
        for item in &self.items {
            if matches!(item, Item::Spacer) {
                spacers += 1;
            } else {
                fixed += self.content_width(cx.fonts, item, pad);
            }
        }
        let spacer_w = if spacers > 0 {
            ((w - fixed) / spacers as f32).max(0.0)
        } else {
            0.0
        };

        let mut x = cx.theme.spacing.sm;
        // Two passes would borrow-conflict; compute widths first.
        let widths: Vec<f32> = self
            .items
            .iter()
            .map(|it| {
                if matches!(it, Item::Spacer) {
                    spacer_w
                } else {
                    self.content_width(cx.fonts, it, pad)
                }
            })
            .collect();
        for (item, iw) in self.items.iter_mut().zip(widths) {
            let r = Rect::from_xywh(x, 4.0, iw, self.bar_h - 8.0);
            match item {
                Item::Button { rect, .. }
                | Item::Toggle { rect, .. }
                | Item::Separator { rect } => *rect = r,
                Item::Spacer => {}
            }
            x += iw;
        }

        constraints.constrain(Size::new(w, self.bar_h))
    }

    fn paint(&mut self, cx: &mut PaintCx<'_>, bounds: Rect, scene: &mut Scene) {
        let p = &cx.theme.palette;
        scene.rect(bounds, p.surface);
        let line_h = cx.fonts.line_height(self.font_size, FontId::Ui);
        let pad = cx.theme.spacing.md;

        for (i, item) in self.items.iter().enumerate() {
            match item {
                Item::Button {
                    icon, label, rect, ..
                } => {
                    let abs = super::absolute(bounds, *rect);
                    let bg = if self.pressed == Some(i) {
                        Some(p.active)
                    } else if self.hovered == Some(i) {
                        Some(p.hover)
                    } else {
                        None
                    };
                    if let Some(bg) = bg {
                        scene.rounded_rect(abs, bg, cx.theme.radius.sm);
                    }
                    draw_content(
                        scene,
                        cx,
                        abs,
                        icon,
                        label,
                        p.text,
                        self.font_size,
                        line_h,
                        pad,
                    );
                }
                Item::Toggle {
                    icon,
                    label,
                    value,
                    rect,
                } => {
                    let abs = super::absolute(bounds, *rect);
                    let on = value.get();
                    if on {
                        scene.rounded_rect(abs, p.selection, cx.theme.radius.sm);
                    } else if self.hovered == Some(i) {
                        scene.rounded_rect(abs, p.hover, cx.theme.radius.sm);
                    }
                    let color = if on { p.accent } else { p.text };
                    draw_content(
                        scene,
                        cx,
                        abs,
                        icon,
                        label,
                        color,
                        self.font_size,
                        line_h,
                        pad,
                    );
                }
                Item::Separator { rect } => {
                    let abs = super::absolute(bounds, *rect);
                    scene.rect(
                        Rect::from_xywh(abs.center().x, abs.top() + 3.0, 1.0, abs.height() - 6.0),
                        p.border,
                    );
                }
                Item::Spacer => {}
            }
        }
    }

    fn event(&mut self, _cx: &mut EventCx<'_>, bounds: Rect, event: &InputEvent) {
        let hit = |items: &[Item], pos: Point| -> Option<usize> {
            items.iter().enumerate().find_map(|(i, it)| {
                let rect = match it {
                    Item::Button { rect, .. } | Item::Toggle { rect, .. } => *rect,
                    _ => return None,
                };
                super::absolute(bounds, rect).contains(pos).then_some(i)
            })
        };

        match event {
            InputEvent::PointerMoved { pos } => self.hovered = hit(&self.items, *pos),
            InputEvent::PointerLeft => self.hovered = None,
            InputEvent::PointerPressed {
                pos,
                button: PointerButton::Primary,
            } => {
                self.pressed = hit(&self.items, *pos);
            }
            InputEvent::PointerReleased {
                pos,
                button: PointerButton::Primary,
            } => {
                if let Some(i) = self.pressed.take() {
                    if hit(&self.items, *pos) == Some(i) {
                        match &mut self.items[i] {
                            Item::Button { on, .. } => on(),
                            Item::Toggle { value, .. } => value.update(|v| *v = !*v),
                            _ => {}
                        }
                    }
                }
            }
            _ => {}
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn draw_content(
    scene: &mut Scene,
    cx: &PaintCx<'_>,
    abs: Rect,
    icon: &Option<Icon>,
    label: &Option<String>,
    color: baseui_core::Color,
    font_size: f32,
    line_h: f32,
    pad: f32,
) {
    let ty = abs.top() + (abs.height() - line_h) * 0.5;
    let mut x = abs.left() + pad;
    if let Some(icon) = icon {
        scene.text_font(
            Point::new(x, ty),
            icon.ch().to_string(),
            font_size,
            color,
            icon.font_id(),
        );
        x += cx.fonts.char_advance(icon.ch(), font_size, icon.font_id());
        if label.is_some() {
            x += pad * 0.5;
        }
    }
    if let Some(label) = label {
        scene.text(Point::new(x, ty), label.clone(), font_size, color);
    }
}
