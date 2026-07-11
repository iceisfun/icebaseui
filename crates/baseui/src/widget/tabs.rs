//! [`TabView`] — a tab strip that swaps which child widget is shown.
//!
//! The strip can sit **on top** ([`TabStrip::Top`], horizontal, icon + text) or
//! **down the left edge** ([`TabStrip::Left`], a vertical icon-only rail — the
//! layout Blender's Properties editor uses, where each icon selects which
//! property pane is shown).
//!
//! Only the selected tab's content is laid out, painted, and sent events, so
//! keeping many panes around is cheap.

use baseui_core::paint::{RectShape, Scene};
use baseui_core::{Point, Rect, Size};

use super::{EventCx, LayoutCx, PaintCx, Widget, absolute};
use crate::event::{InputEvent, PointerButton};
use crate::icon::Icon;
use crate::layout::Constraints;
use crate::text::FontId;

/// Where the tab strip sits.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum TabStrip {
    /// Horizontal strip across the top; tabs show icon + title.
    Top,
    /// Vertical icon rail down the left edge; tabs show their icon only.
    Left,
}

/// Width of the vertical icon rail, and the height of one of its tabs.
const RAIL_W: f32 = 40.0;
const RAIL_ITEM_H: f32 = 38.0;

struct Tab {
    title: String,
    icon: Option<Icon>,
    content: Box<dyn Widget>,
    header_rect: Rect,
}

/// A tabbed container. The selected tab's content fills the area beside the strip.
pub struct TabView {
    tabs: Vec<Tab>,
    selected: usize,
    hovered: Option<usize>,
    font_size: f32,
    icon_size: f32,
    strip: TabStrip,
    /// Header height (`Top`) or rail width (`Left`).
    strip_size: f32,
    content_rect: Rect,
    persist_key: Option<String>,
}

impl TabView {
    /// An empty view with the strip on top. Add tabs with [`TabView::tab`].
    pub fn new() -> Self {
        TabView {
            tabs: Vec::new(),
            selected: 0,
            hovered: None,
            font_size: 13.0,
            icon_size: 17.0,
            strip: TabStrip::Top,
            strip_size: 30.0,
            content_rect: Rect::ZERO,
            persist_key: None,
        }
    }

    /// Put the tabs in a vertical icon rail down the left edge (Blender-style).
    /// Tabs should have icons; a tab without one falls back to its first letter.
    pub fn vertical(mut self) -> Self {
        self.strip = TabStrip::Left;
        self
    }

    /// Choose where the tab strip sits.
    pub fn strip(mut self, strip: TabStrip) -> Self {
        self.strip = strip;
        self
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

    /// Add a tab with an icon (shown alone in a vertical rail).
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

    /// Persist the selected tab index under `key` between runs.
    pub fn persist(mut self, key: impl Into<String>) -> Self {
        self.persist_key = Some(key.into());
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

        match self.strip {
            TabStrip::Top => {
                let line_h = cx.fonts.line_height(self.font_size, FontId::Ui);
                self.strip_size = line_h + cx.theme.spacing.md;
                let pad = cx.theme.spacing.md;
                let mut x = 0.0;
                for tab in &mut self.tabs {
                    let mut tw = cx
                        .fonts
                        .measure(&tab.title, self.font_size, FontId::Ui)
                        .width;
                    if let Some(icon) = tab.icon {
                        tw += cx
                            .fonts
                            .char_advance(icon.ch(), self.font_size, icon.font_id())
                            + pad * 0.5;
                    }
                    tw += pad * 2.0;
                    tab.header_rect = Rect::from_xywh(x, 0.0, tw, self.strip_size);
                    x += tw;
                }
                self.content_rect =
                    Rect::from_xywh(0.0, self.strip_size, w, (h - self.strip_size).max(0.0));
            }
            TabStrip::Left => {
                self.strip_size = RAIL_W * crate::text::scale();
                let item_h = RAIL_ITEM_H * crate::text::scale();
                for (i, tab) in self.tabs.iter_mut().enumerate() {
                    tab.header_rect =
                        Rect::from_xywh(0.0, i as f32 * item_h, self.strip_size, item_h);
                }
                self.content_rect =
                    Rect::from_xywh(self.strip_size, 0.0, (w - self.strip_size).max(0.0), h);
            }
        }

        if let Some(tab) = self.tabs.get_mut(self.selected) {
            tab.content
                .layout(cx, Constraints::loose(self.content_rect.size));
        }

        constraints.constrain(Size::new(w, h))
    }

    fn paint(&mut self, cx: &mut PaintCx<'_>, bounds: Rect, scene: &mut Scene) {
        let p = &cx.theme.palette;

        // Strip background.
        let strip_rect = match self.strip {
            TabStrip::Top => {
                Rect::from_xywh(bounds.left(), bounds.top(), bounds.width(), self.strip_size)
            }
            TabStrip::Left => Rect::from_xywh(
                bounds.left(),
                bounds.top(),
                self.strip_size,
                bounds.height(),
            ),
        };
        scene.rect(strip_rect, p.surface_variant);

        for (i, tab) in self.tabs.iter().enumerate() {
            let hr = absolute(bounds, tab.header_rect);
            let selected = i == self.selected;

            match self.strip {
                TabStrip::Top => {
                    if selected {
                        scene.push_rect(RectShape::fill(hr, p.surface));
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
                        tx += cx
                            .fonts
                            .char_advance(icon.ch(), self.font_size, icon.font_id())
                            + cx.theme.spacing.sm;
                    }
                    scene.text(Point::new(tx, ty), tab.title.clone(), self.font_size, color);
                }
                TabStrip::Left => {
                    // Selected tab reads as continuous with the content pane, with
                    // an accent bar on its outer edge.
                    if selected {
                        scene.push_rect(RectShape::fill(hr, p.surface));
                        scene.rect(
                            Rect::from_xywh(hr.left(), hr.top(), 2.0, hr.height()),
                            p.accent,
                        );
                    } else if self.hovered == Some(i) {
                        scene.push_rect(RectShape::fill(hr, p.hover));
                    }

                    // Icon-only (fall back to the title's first letter).
                    let (ch, font) = match tab.icon {
                        Some(icon) => (icon.ch(), icon.font_id()),
                        None => (tab.title.chars().next().unwrap_or('?'), FontId::Ui),
                    };
                    let color = if selected { p.text } else { p.text_muted };
                    let adv = cx.fonts.char_advance(ch, self.icon_size, font);
                    let line_h = cx.fonts.line_height(self.icon_size, font);
                    scene.text_font(
                        Point::new(hr.center().x - adv * 0.5, hr.center().y - line_h * 0.5),
                        ch.to_string(),
                        self.icon_size,
                        color,
                        font,
                    );
                }
            }
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
        // Content FIRST: it may own a popup (overlay layer) floating over the tab
        // strip, which must consume the event before we read it as a tab click.
        let cr = absolute(bounds, self.content_rect);
        if let Some(tab) = self.tabs.get_mut(self.selected) {
            tab.content.event(cx, cr, event);
        }

        if cx.is_consumed() {
            self.hovered = None;
            return;
        }

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
                    cx.consume();
                }
            }
            _ => {}
        }
    }

    fn persist_save(&self, store: &mut crate::persist::Store) {
        if let Some(key) = &self.persist_key {
            store.set(key.clone(), &self.selected);
        }
        for tab in &self.tabs {
            tab.content.persist_save(store);
        }
    }

    fn persist_restore(&mut self, store: &crate::persist::Store) {
        if let Some(key) = &self.persist_key {
            if let Some(i) = store.get::<usize>(key) {
                if i < self.tabs.len() {
                    self.selected = i;
                }
            }
        }
        for tab in &mut self.tabs {
            tab.content.persist_restore(store);
        }
    }
}
