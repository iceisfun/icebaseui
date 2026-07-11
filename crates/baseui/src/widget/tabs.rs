//! [`TabView`] — a tab strip that swaps which child widget is shown.
//!
//! Each tab owns a content [`Widget`]; only the selected tab's content is laid
//! out, painted, and sent events. Tabs may carry a leading glyph [`Icon`]
//! (Blender's Properties editor uses icon tabs). Place inside a
//! [`Split`](super::Split) pane or any bounded region.

use baseui_core::paint::{RectShape, Scene};
use baseui_core::{Point, Rect, Size};

use super::{EventCx, LayoutCx, PaintCx, Widget, absolute};
use crate::event::{InputEvent, PointerButton};
use crate::icon::Icon;
use crate::layout::Constraints;
use crate::text::FontId;

struct Tab {
    title: String,
    icon: Option<Icon>,
    content: Box<dyn Widget>,
    header_rect: Rect,
}

/// A tabbed container. The selected tab's content fills the area below the tab
/// strip.
pub struct TabView {
    tabs: Vec<Tab>,
    selected: usize,
    hovered: Option<usize>,
    font_size: f32,
    header_h: f32,
    content_rect: Rect,
}

impl TabView {
    pub fn new() -> Self {
        TabView {
            tabs: Vec::new(),
            selected: 0,
            hovered: None,
            font_size: 13.0,
            header_h: 30.0,
            content_rect: Rect::ZERO,
        }
    }

    /// Add a text tab.
    pub fn tab(mut self, title: impl Into<String>, content: impl Widget + 'static) -> Self {
        self.tabs.push(Tab {
            title: title.into(),
            icon: None,
            content: Box::new(content),
            header_rect: Rect::ZERO,
        });
        self
    }

    /// Add a tab with a leading icon.
    pub fn tab_icon(
        mut self,
        icon: Icon,
        title: impl Into<String>,
        content: impl Widget + 'static,
    ) -> Self {
        self.tabs.push(Tab {
            title: title.into(),
            icon: Some(icon),
            content: Box::new(content),
            header_rect: Rect::ZERO,
        });
        self
    }

    /// Select an initial tab index.
    pub fn selected(mut self, index: usize) -> Self {
        self.selected = index;
        self
    }
}

impl Default for TabView {
    fn default() -> Self {
        TabView::new()
    }
}

impl Widget for TabView {
    fn layout(&mut self, cx: &mut LayoutCx<'_>, constraints: Constraints) -> Size {
        let w = if constraints.max.width.is_finite() {
            constraints.max.width
        } else {
            360.0
        };
        let h = if constraints.max.height.is_finite() {
            constraints.max.height
        } else {
            400.0
        };
        let line_h = cx.fonts.line_height(self.font_size, FontId::Ui);
        self.header_h = line_h + cx.theme.spacing.md;
        let pad = cx.theme.spacing.md;

        let mut x = 0.0;
        for tab in &mut self.tabs {
            let tw = {
                // borrow split: measure without &mut self
                let mut w = cx.fonts.measure(&tab.title, self.font_size, FontId::Ui).width;
                if let Some(icon) = tab.icon {
                    w += cx.fonts.char_advance(icon.ch(), self.font_size, icon.font_id())
                        + pad * 0.5;
                }
                w + pad * 2.0
            };
            tab.header_rect = Rect::from_xywh(x, 0.0, tw, self.header_h);
            x += tw;
        }

        self.content_rect = Rect::from_xywh(0.0, self.header_h, w, (h - self.header_h).max(0.0));
        if let Some(tab) = self.tabs.get_mut(self.selected) {
            tab.content
                .layout(cx, Constraints::loose(self.content_rect.size));
        }

        constraints.constrain(Size::new(w, h))
    }

    fn paint(&mut self, cx: &mut PaintCx<'_>, bounds: Rect, scene: &mut Scene) {
        let p = &cx.theme.palette;

        // Strip background + bottom divider.
        let strip = Rect::from_xywh(bounds.left(), bounds.top(), bounds.width(), self.header_h);
        scene.rect(strip, p.surface_variant);

        for (i, tab) in self.tabs.iter().enumerate() {
            let hr = absolute(bounds, tab.header_rect);
            let selected = i == self.selected;

            if selected {
                scene.push_rect(RectShape::fill(hr, p.surface));
                // Accent underline.
                scene.rect(
                    Rect::from_xywh(hr.left(), hr.bottom() - 2.0, hr.width(), 2.0),
                    p.accent,
                );
            } else if self.hovered == Some(i) {
                scene.push_rect(RectShape::fill(hr, p.hover));
            }

            let color = if selected { p.text } else { p.text_muted };
            let line_h = cx.fonts.line_height(self.font_size, FontId::Ui);
            let ty = hr.top() + (hr.height() - line_h) * 0.5;
            let mut tx = hr.left() + cx.theme.spacing.md;
            if let Some(icon) = tab.icon {
                scene.text_font(
                    Point::new(tx, ty),
                    icon.ch().to_string(),
                    self.font_size,
                    color,
                    icon.font_id(),
                );
                tx += cx.fonts.char_advance(icon.ch(), self.font_size, icon.font_id())
                    + cx.theme.spacing.sm;
            }
            scene.text(Point::new(tx, ty), tab.title.clone(), self.font_size, color);
        }

        // Selected content.
        if let Some(tab) = self.tabs.get_mut(self.selected) {
            let cr = absolute(bounds, self.content_rect);
            scene.push_clip(cr);
            tab.content.paint(cx, cr, scene);
            scene.pop_clip();
        }
    }

    fn event(&mut self, cx: &mut EventCx<'_>, bounds: Rect, event: &InputEvent) {
        match event {
            InputEvent::PointerMoved { pos } => {
                self.hovered = self
                    .tabs
                    .iter()
                    .position(|t| absolute(bounds, t.header_rect).contains(*pos));
            }
            InputEvent::PointerLeft => self.hovered = None,
            InputEvent::PointerPressed {
                pos,
                button: PointerButton::Primary,
            } => {
                if let Some(i) = self
                    .tabs
                    .iter()
                    .position(|t| absolute(bounds, t.header_rect).contains(*pos))
                {
                    self.selected = i;
                    return;
                }
            }
            _ => {}
        }

        // Forward to the selected content.
        let cr = absolute(bounds, self.content_rect);
        if let Some(tab) = self.tabs.get_mut(self.selected) {
            tab.content.event(cx, cr, event);
        }
    }
}
