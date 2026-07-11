//! [`MenuBar`] and [`Menu`] — a top menu bar with dropdown menus.
//!
//! Clicking a title opens its dropdown, drawn in the [`Scene`] overlay layer so
//! it floats above the rest of the UI. Clicking an item runs its command and
//! closes the menu; clicking anywhere else dismisses it. Dismiss works because
//! container widgets forward pointer events to all children, so the bar sees
//! clicks that land outside it.

use baseui_core::paint::{RectShape, Scene};
use baseui_core::{Insets, Point, Rect, Size};

use super::{EventCx, LayoutCx, PaintCx, Widget};
use crate::event::{InputEvent, Key, PointerButton};
use crate::icon::{Icon, glyphs};
use crate::layout::Constraints;
use crate::text::FontId;

type Action = Box<dyn FnMut()>;

enum Entry {
    Item {
        icon: Option<Icon>,
        label: String,
        on: Action,
        /// Optional right-aligned "options" button (Maya-style: e.g. Create ▸
        /// Cube with a gear to set subdivisions), with its own handler.
        options: Option<Action>,
    },
    Separator,
}

/// Width reserved on the right of an item for its options button.
const OPTIONS_W: f32 = 26.0;

/// One top-level menu (a title plus its dropdown entries).
pub struct Menu {
    title: String,
    entries: Vec<Entry>,
}

impl Menu {
    pub fn new(title: impl Into<String>) -> Self {
        Menu {
            title: title.into(),
            entries: Vec::new(),
        }
    }

    /// Add a command item.
    pub fn item(mut self, label: impl Into<String>, on: impl FnMut() + 'static) -> Self {
        self.entries.push(Entry::Item {
            icon: None,
            label: label.into(),
            on: Box::new(on),
            options: None,
        });
        self
    }

    /// Add a command item with a leading icon.
    pub fn item_icon(
        mut self,
        icon: Icon,
        label: impl Into<String>,
        on: impl FnMut() + 'static,
    ) -> Self {
        self.entries.push(Entry::Item {
            icon: Some(icon),
            label: label.into(),
            on: Box::new(on),
            options: None,
        });
        self
    }

    /// Add an item with a leading icon and a right-aligned options button
    /// (Maya's "Create ▸ Cube ▸ ⚙" pattern). `options` runs when the gear is
    /// clicked; `on` runs for the rest of the row.
    pub fn item_options(
        mut self,
        icon: Icon,
        label: impl Into<String>,
        on: impl FnMut() + 'static,
        options: impl FnMut() + 'static,
    ) -> Self {
        self.entries.push(Entry::Item {
            icon: Some(icon),
            label: label.into(),
            on: Box::new(on),
            options: Some(Box::new(options)),
        });
        self
    }

    /// Add a separator line.
    pub fn separator(mut self) -> Self {
        self.entries.push(Entry::Separator);
        self
    }
}

const SEPARATOR_H: f32 = 7.0;
const MENU_MIN_W: f32 = 150.0;

/// The top menu bar.
pub struct MenuBar {
    menus: Vec<Menu>,
    open: Option<usize>,
    hovered_title: Option<usize>,
    hovered_item: Option<usize>,
    font_size: f32,
    bar_h: f32,
    title_rects: Vec<Rect>,
}

impl MenuBar {
    pub fn new() -> Self {
        MenuBar {
            menus: Vec::new(),
            open: None,
            hovered_title: None,
            hovered_item: None,
            font_size: 14.0,
            bar_h: 30.0,
            title_rects: Vec::new(),
        }
    }

    pub fn menu(mut self, menu: Menu) -> Self {
        self.menus.push(menu);
        self
    }

    /// Open menu `i`. An open menu is modal for the keyboard: it clears focus so
    /// a focused text field stops receiving keystrokes behind it.
    fn open_menu(&mut self, index: usize) {
        self.open = Some(index);
        self.hovered_item = None;
        crate::popup::set_open(true);
    }

    fn close_menu(&mut self) {
        if self.open.take().is_some() {
            self.hovered_item = None;
            crate::popup::set_open(false);
        }
    }

    /// Geometry of the open dropdown: (panel rect, per-entry rects). Entry rects
    /// align with `menu.entries` (separators included). All absolute.
    fn dropdown(&self, fonts: &crate::text::Fonts, bounds: Rect, index: usize) -> (Rect, Vec<Rect>) {
        let menu = &self.menus[index];
        let line_h = fonts.line_height(self.font_size, FontId::Ui);
        let item_h = line_h + 8.0;
        let pad = 10.0;

        let mut width = MENU_MIN_W;
        for entry in &menu.entries {
            if let Entry::Item {
                icon, label, options, ..
            } = entry
            {
                let mut w = fonts.measure(label, self.font_size, FontId::Ui).width + pad * 2.0;
                if let Some(icon) = icon {
                    w += fonts.char_advance(icon.ch(), self.font_size, icon.font_id()) + 8.0;
                }
                if options.is_some() {
                    w += OPTIONS_W;
                }
                width = width.max(w);
            }
        }

        let title = self.title_rects[index];
        let panel_x = bounds.left() + title.left();
        let mut y = bounds.bottom();
        let start_y = y;

        let mut rects = Vec::with_capacity(menu.entries.len());
        for entry in &menu.entries {
            let h = match entry {
                Entry::Item { .. } => item_h,
                Entry::Separator => SEPARATOR_H,
            };
            rects.push(Rect::from_xywh(panel_x, y, width, h));
            y += h;
        }
        let panel = Rect::from_xywh(panel_x, start_y, width, y - start_y);
        (panel, rects)
    }
}

impl Default for MenuBar {
    fn default() -> Self {
        MenuBar::new()
    }
}

impl Widget for MenuBar {
    fn layout(&mut self, cx: &mut LayoutCx<'_>, constraints: Constraints) -> Size {
        let line_h = cx.fonts.line_height(self.font_size, FontId::Ui);
        self.bar_h = line_h + cx.theme.spacing.sm * 2.0;
        let w = if constraints.max.width.is_finite() {
            constraints.max.width
        } else {
            800.0
        };

        self.title_rects.clear();
        let mut x = cx.theme.spacing.sm;
        for menu in &self.menus {
            let tw = cx.fonts.measure(&menu.title, self.font_size, FontId::Ui).width
                + cx.theme.spacing.md * 2.0;
            self.title_rects.push(Rect::from_xywh(x, 0.0, tw, self.bar_h));
            x += tw;
        }

        constraints.constrain(Size::new(w, self.bar_h))
    }

    fn paint(&mut self, cx: &mut PaintCx<'_>, bounds: Rect, scene: &mut Scene) {
        let p = &cx.theme.palette;
        scene.rect(bounds, p.surface);

        let line_h = cx.fonts.line_height(self.font_size, FontId::Ui);
        for (i, menu) in self.menus.iter().enumerate() {
            let tr = super::absolute(bounds, self.title_rects[i]);
            if self.open == Some(i) {
                scene.rounded_rect(tr, p.active, cx.theme.radius.sm);
            } else if self.hovered_title == Some(i) {
                scene.rounded_rect(tr, p.hover, cx.theme.radius.sm);
            }
            scene.text(
                Point::new(tr.left() + cx.theme.spacing.md, tr.top() + (tr.height() - line_h) * 0.5),
                menu.title.clone(),
                self.font_size,
                p.text,
            );
        }

        // Open dropdown, in the overlay layer.
        if let Some(i) = self.open {
            let (panel, rects) = self.dropdown(cx.fonts, bounds, i);
            scene.begin_overlay();
            scene.push_rect(
                RectShape::fill(panel, p.surface)
                    .with_corner_radius(cx.theme.radius.md)
                    .with_border(1.0, p.border),
            );
            for (k, entry) in self.menus[i].entries.iter().enumerate() {
                let r = rects[k];
                match entry {
                    Entry::Item {
                        icon, label, options, ..
                    } => {
                        if self.hovered_item == Some(k) {
                            scene.rounded_rect(
                                r.shrink(Insets::symmetric(3.0, 1.0)),
                                p.hover,
                                cx.theme.radius.sm,
                            );
                        }
                        let ty = r.top() + (r.height() - line_h) * 0.5;
                        let mut tx = r.left() + 10.0;
                        if let Some(icon) = icon {
                            scene.text_font(
                                Point::new(tx, ty),
                                icon.ch().to_string(),
                                self.font_size,
                                p.text,
                                icon.font_id(),
                            );
                            tx += cx.fonts.char_advance(icon.ch(), self.font_size, icon.font_id())
                                + 8.0;
                        }
                        scene.text(Point::new(tx, ty), label.clone(), self.font_size, p.text);
                        if options.is_some() {
                            // Right-aligned options gear.
                            let gx = r.right() - OPTIONS_W + 6.0;
                            scene.text_font(
                                Point::new(gx, ty),
                                glyphs::GEAR.ch().to_string(),
                                self.font_size,
                                p.text_muted,
                                glyphs::GEAR.font_id(),
                            );
                        }
                    }
                    Entry::Separator => {
                        let y = r.center().y;
                        scene.rect(
                            Rect::from_xywh(r.left() + 6.0, y, r.width() - 12.0, 1.0),
                            p.border,
                        );
                    }
                }
            }
            scene.end_overlay();
        }
    }

    fn event(&mut self, cx: &mut EventCx<'_>, bounds: Rect, event: &InputEvent) {
        match event {
            InputEvent::PointerMoved { pos } => {
                self.hovered_title = self
                    .title_rects
                    .iter()
                    .position(|r| super::absolute(bounds, *r).contains(*pos));
                self.hovered_item = None;
                if let Some(i) = self.open {
                    let (panel, rects) = self.dropdown(cx.fonts, bounds, i);
                    for (k, r) in rects.iter().enumerate() {
                        if matches!(self.menus[i].entries[k], Entry::Item { .. }) && r.contains(*pos) {
                            self.hovered_item = Some(k);
                        }
                    }
                    // Swallow moves over the open dropdown so widgets beneath it
                    // don't hover through the popup.
                    if panel.contains(*pos) {
                        cx.consume();
                    }
                }
                if bounds.contains(*pos) {
                    cx.consume();
                }
            }
            InputEvent::PointerLeft => {
                self.hovered_title = None;
                self.hovered_item = None;
            }
            InputEvent::PointerPressed {
                pos,
                button: PointerButton::Primary,
            } => {
                // Title click: toggle that menu.
                if let Some(i) = self
                    .title_rects
                    .iter()
                    .position(|r| super::absolute(bounds, *r).contains(*pos))
                {
                    if self.open == Some(i) {
                        self.close_menu();
                    } else {
                        self.open_menu(i);
                    }
                    cx.consume();
                    return;
                }
                // Item click, or dismiss (both while a menu is open consume the
                // click so it doesn't fall through to widgets below).
                if let Some(i) = self.open {
                    let (panel, rects) = self.dropdown(cx.fonts, bounds, i);
                    let mut hit = None;
                    for (k, r) in rects.iter().enumerate() {
                        if matches!(self.menus[i].entries[k], Entry::Item { .. }) && r.contains(*pos)
                        {
                            // Right options-gear zone vs the rest of the row.
                            let on_options = pos.x >= r.right() - OPTIONS_W;
                            hit = Some((k, on_options));
                            break;
                        }
                    }
                    if let Some((k, on_options)) = hit {
                        if let Entry::Item { on, options, .. } = &mut self.menus[i].entries[k] {
                            match (on_options, options) {
                                (true, Some(opts)) => opts(),
                                _ => on(),
                            }
                        }
                    }
                    if panel.contains(*pos) || hit.is_some() {
                        cx.consume();
                    }
                    self.close_menu(); // click on item or outside both dismiss
                }
            }
            // Escape closes an open menu (it is modal for the keyboard).
            InputEvent::Key {
                key: Key::Escape,
                pressed: true,
                ..
            } => {
                if self.open.is_some() {
                    self.close_menu();
                    cx.consume();
                }
            }
            _ => {}
        }
    }
}
