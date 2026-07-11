//! [`StatusBar`] and [`StatusItem`] — the bottom status strip.
//!
//! Applications and plugins contribute independent items, left- or right-aligned,
//! each an optional icon plus static or reactive text (e.g. `FPS 144`,
//! `Git main`). Reactive items recompute their text each frame, so a signal
//! change updates the status line.

use std::cell::RefCell;

use baseui_core::paint::Scene;
use baseui_core::{Color, Point, Rect, Size};

use super::{LayoutCx, PaintCx, Widget};
use crate::icon::Icon;
use crate::layout::Constraints;
use crate::text::FontId;

thread_local! {
    /// Items contributed by plugins/scripts, merged in by every [`StatusBar`].
    static CONTRIBUTED: RefCell<Vec<StatusItem>> = const { RefCell::new(Vec::new()) };
}

/// Contribute a status item from a plugin or script.
///
/// Applications build their own items with [`StatusBar::item`]; plugins and Lua
/// scripts call this instead, and every `StatusBar` renders them alongside its
/// own (SOW: "Applications and plugins may contribute independent status items").
pub fn contribute(item: StatusItem) {
    CONTRIBUTED.with(|c| c.borrow_mut().push(item));
}

enum Text {
    Static(String),
    Dynamic(Box<dyn FnMut() -> String>),
}

/// Which side of the bar an item sits on.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Side {
    Left,
    Right,
}

/// A single status-bar contribution.
pub struct StatusItem {
    icon: Option<Icon>,
    text: Text,
    color: Option<Color>,
    side: Side,
}

impl StatusItem {
    /// An item with fixed text.
    pub fn new(text: impl Into<String>) -> Self {
        StatusItem {
            icon: None,
            text: Text::Static(text.into()),
            color: None,
            side: Side::Left,
        }
    }

    /// An item whose text is recomputed each frame (typically reading a signal).
    pub fn dynamic(f: impl FnMut() -> String + 'static) -> Self {
        StatusItem {
            icon: None,
            text: Text::Dynamic(Box::new(f)),
            color: None,
            side: Side::Left,
        }
    }

    /// Add a leading icon.
    pub fn icon(mut self, icon: Icon) -> Self {
        self.icon = Some(icon);
        self
    }

    /// Override the text color.
    pub fn color(mut self, color: Color) -> Self {
        self.color = Some(color);
        self
    }

    /// Align this item to the right side of the bar.
    pub fn right(mut self) -> Self {
        self.side = Side::Right;
        self
    }

    fn resolve(&mut self) -> String {
        match &mut self.text {
            Text::Static(s) => s.clone(),
            Text::Dynamic(f) => f(),
        }
    }
}

/// The bottom status bar.
pub struct StatusBar {
    items: Vec<StatusItem>,
    font_size: f32,
    bar_h: f32,
}

impl StatusBar {
    pub fn new() -> Self {
        StatusBar {
            items: Vec::new(),
            font_size: 12.0,
            bar_h: 24.0,
        }
    }

    pub fn item(mut self, item: StatusItem) -> Self {
        self.items.push(item);
        self
    }
}

impl Default for StatusBar {
    fn default() -> Self {
        StatusBar::new()
    }
}

impl Widget for StatusBar {
    fn layout(&mut self, cx: &mut LayoutCx<'_>, constraints: Constraints) -> Size {
        let line_h = cx.fonts.line_height(self.font_size, FontId::Ui);
        self.bar_h = line_h + cx.theme.spacing.sm * 2.0;
        let w = if constraints.max.width.is_finite() {
            constraints.max.width
        } else {
            800.0
        };
        constraints.constrain(Size::new(w, self.bar_h))
    }

    fn paint(&mut self, cx: &mut PaintCx<'_>, bounds: Rect, scene: &mut Scene) {
        let p = &cx.theme.palette;
        scene.rect(bounds, p.surface);
        scene.rect(Rect::from_xywh(bounds.left(), bounds.top(), bounds.width(), 1.0), p.border);

        let line_h = cx.fonts.line_height(self.font_size, FontId::Ui);
        let ty = bounds.top() + (bounds.height() - line_h) * 0.5;
        let gap = cx.theme.spacing.lg;
        let icon_gap = cx.theme.spacing.xs;

        let mut left_x = bounds.left() + cx.theme.spacing.md;
        let mut right_x = bounds.right() - cx.theme.spacing.md;

        // Resolve widths up front (dynamic text needs &mut).
        let mut resolved: Vec<(String, Option<Icon>, Color, Side, f32)> =
            Vec::with_capacity(self.items.len());
        let resolve_into =
            |item: &mut StatusItem, out: &mut Vec<(String, Option<Icon>, Color, Side, f32)>| {
                let s = item.resolve();
                let color = item.color.unwrap_or(p.text_muted);
                let mut w = cx.fonts.measure(&s, self.font_size, FontId::Ui).width;
                if let Some(icon) = item.icon {
                    w +=
                        cx.fonts.char_advance(icon.ch(), self.font_size, icon.font_id()) + icon_gap;
                }
                out.push((s, item.icon, color, item.side, w));
            };

        for item in &mut self.items {
            resolve_into(item, &mut resolved);
        }

        // Plugin/script contributions. Taken out while resolving (a dynamic item
        // could call `contribute` re-entrantly), then merged back.
        let mut contributed = CONTRIBUTED.with(|c| std::mem::take(&mut *c.borrow_mut()));
        for item in &mut contributed {
            resolve_into(item, &mut resolved);
        }
        CONTRIBUTED.with(|c| {
            let mut slot = c.borrow_mut();
            contributed.append(&mut slot);
            *slot = contributed;
        });

        for (s, icon, color, side, w) in resolved {
            let x0 = match side {
                Side::Left => {
                    let x = left_x;
                    left_x += w + gap;
                    x
                }
                Side::Right => {
                    right_x -= w;
                    let x = right_x;
                    right_x -= gap;
                    x
                }
            };
            let mut x = x0;
            if let Some(icon) = icon {
                scene.text_font(Point::new(x, ty), icon.ch().to_string(), self.font_size, color, icon.font_id());
                x += cx.fonts.char_advance(icon.ch(), self.font_size, icon.font_id()) + icon_gap;
            }
            scene.text(Point::new(x, ty), s, self.font_size, color);
        }
    }
}
