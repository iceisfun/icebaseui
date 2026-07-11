//! [`PopupMenu`] — the one popup list used by dropdown menus, combo boxes, and
//! right-click context menus.
//!
//! It owns the parts every popup needs and every one of them got slightly wrong
//! when written separately:
//!
//! - **placement** that stays on screen ([`popup::place`](crate::popup::place)):
//!   opens below its anchor, flips above when there is no room, clamps to the
//!   edges. A context menu is just a popup anchored to a zero-size rect at the
//!   click point.
//! - **overlay painting**, so it floats above the rest of the tree.
//! - **event consumption**, so clicks and hover don't leak through to whatever
//!   is underneath.
//! - **keyboard modality** ([`popup`](crate::popup)): opening clears focus and
//!   suppresses shortcuts; Escape closes.
//!
//! Owners keep their own state and map an [`Activation`] index back to an action.

use baseui_core::paint::{RectShape, Scene};
use baseui_core::{Insets, Point, Rect, Size};

use super::{EventCx, PaintCx};
use crate::event::{InputEvent, Key, PointerButton};
use crate::icon::{Icon, glyphs};
use crate::popup;
use crate::text::FontId;

const MIN_W: f32 = 150.0;
const SEPARATOR_H: f32 = 7.0;
/// Width reserved on the right of an item for its options button.
const OPTIONS_W: f32 = 26.0;

/// One entry in a [`PopupMenu`].
#[derive(Clone, Default)]
pub struct MenuItemSpec {
    pub label: String,
    pub icon: Option<Icon>,
    /// Right-aligned shortcut hint, e.g. `"Ctrl+S"`.
    pub shortcut: Option<String>,
    pub enabled: bool,
    pub separator: bool,
    /// Show a right-aligned options gear that activates separately (Maya-style).
    pub has_options: bool,
}

impl MenuItemSpec {
    pub fn new(label: impl Into<String>) -> Self {
        MenuItemSpec {
            label: label.into(),
            enabled: true,
            ..Default::default()
        }
    }

    pub fn separator() -> Self {
        MenuItemSpec {
            separator: true,
            ..Default::default()
        }
    }

    pub fn icon(mut self, icon: Icon) -> Self {
        self.icon = Some(icon);
        self
    }

    pub fn shortcut(mut self, shortcut: impl Into<String>) -> Self {
        self.shortcut = Some(shortcut.into());
        self
    }

    pub fn disabled(mut self) -> Self {
        self.enabled = false;
        self
    }

    pub fn with_options(mut self) -> Self {
        self.has_options = true;
        self
    }
}

/// What a click activated: an item index, and whether it hit the options gear.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Activation {
    pub index: usize,
    pub options: bool,
}

/// A popup list, anchored and kept on screen.
#[derive(Default)]
pub struct PopupMenu {
    items: Vec<MenuItemSpec>,
    open: bool,
    anchor: Rect,
    /// Highlighted entry (e.g. a combo box's current value).
    selected: Option<usize>,
    hovered: Option<usize>,
    font_size: f32,
    panel: Rect,
    item_rects: Vec<Rect>,
}

impl PopupMenu {
    pub fn new() -> Self {
        PopupMenu {
            font_size: 14.0,
            ..Default::default()
        }
    }

    pub fn is_open(&self) -> bool {
        self.open
    }

    /// The popup's panel rect (valid once open and laid out).
    pub fn panel(&self) -> Rect {
        self.panel
    }

    /// Highlight an entry (a combo box marks its current value).
    pub fn set_selected(&mut self, index: Option<usize>) {
        self.selected = index;
    }

    /// Open anchored **below** `anchor` — a menu title, or a combo button.
    pub fn open_below(&mut self, anchor: Rect, items: Vec<MenuItemSpec>) {
        self.items = items;
        self.anchor = anchor;
        self.hovered = None;
        self.open = true;
        popup::set_open(true);
    }

    /// Open **at a point** — a right-click context menu.
    pub fn open_at(&mut self, pos: Point, items: Vec<MenuItemSpec>) {
        self.open_below(Rect::new(pos, Size::ZERO), items);
    }

    pub fn close(&mut self) {
        if self.open {
            self.open = false;
            self.hovered = None;
            popup::set_open(false);
        }
    }

    /// Recompute the panel and item rects. Called from both `paint` and `event`,
    /// so hit-testing always matches what is drawn (even on the very first event
    /// after opening, before any paint).
    fn compute(&mut self, fonts: &crate::text::Fonts, screen: Size) {
        let s = crate::text::scale();
        let line_h = fonts.line_height(self.font_size, FontId::Ui);
        let item_h = line_h + 8.0 * s;
        let pad = 10.0 * s;

        let mut width = MIN_W * s;
        for item in &self.items {
            if item.separator {
                continue;
            }
            let mut w = pad * 2.0 + fonts.measure(&item.label, self.font_size, FontId::Ui).width;
            if let Some(icon) = item.icon {
                w += fonts.char_advance(icon.ch(), self.font_size, icon.font_id()) + 8.0 * s;
            }
            if let Some(shortcut) = &item.shortcut {
                w += 20.0 * s + fonts.measure(shortcut, self.font_size - 1.0, FontId::Ui).width;
            }
            if item.has_options {
                w += OPTIONS_W * s;
            }
            width = width.max(w);
        }

        let height: f32 = self
            .items
            .iter()
            .map(|i| if i.separator { SEPARATOR_H * s } else { item_h })
            .sum();

        self.panel = popup::place(self.anchor, Size::new(width, height), screen, 2.0 * s);

        self.item_rects.clear();
        let mut y = self.panel.top();
        for item in &self.items {
            let h = if item.separator {
                SEPARATOR_H * s
            } else {
                item_h
            };
            self.item_rects
                .push(Rect::from_xywh(self.panel.left(), y, width, h));
            y += h;
        }
    }

    /// Item index under `pos`, if it is a selectable (enabled, non-separator) row.
    fn item_at(&self, pos: Point) -> Option<usize> {
        self.item_rects.iter().position(|r| r.contains(pos)).filter(|&i| {
            let item = &self.items[i];
            !item.separator && item.enabled
        })
    }

    /// Draw into the scene's overlay layer.
    pub fn paint(&mut self, cx: &PaintCx<'_>, scene: &mut Scene) {
        if !self.open {
            return;
        }
        self.compute(cx.fonts, cx.screen);

        let p = &cx.theme.palette;
        let s = crate::text::scale();
        let line_h = cx.fonts.line_height(self.font_size, FontId::Ui);

        scene.begin_overlay();
        scene.push_rect(
            RectShape::fill(self.panel, p.surface)
                .with_corner_radius(cx.theme.radius.md)
                .with_border(1.0, p.border),
        );

        for (i, item) in self.items.iter().enumerate() {
            let r = self.item_rects[i];

            if item.separator {
                let y = r.center().y;
                scene.rect(
                    Rect::from_xywh(r.left() + 6.0 * s, y, r.width() - 12.0 * s, 1.0),
                    p.border,
                );
                continue;
            }

            if self.hovered == Some(i) {
                scene.rounded_rect(
                    r.shrink(Insets::symmetric(3.0 * s, 1.0)),
                    p.hover,
                    cx.theme.radius.sm,
                );
            } else if self.selected == Some(i) {
                scene.rounded_rect(
                    r.shrink(Insets::symmetric(3.0 * s, 1.0)),
                    p.selection,
                    cx.theme.radius.sm,
                );
            }

            let color = if item.enabled { p.text } else { p.text_muted };
            let ty = r.top() + (r.height() - line_h) * 0.5;
            let mut tx = r.left() + 10.0 * s;

            if let Some(icon) = item.icon {
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
                    + 8.0 * s;
            }
            scene.text(Point::new(tx, ty), item.label.clone(), self.font_size, color);

            let mut right = r.right() - 10.0 * s;
            if item.has_options {
                let gear = glyphs::GEAR;
                right -= OPTIONS_W * s - 10.0 * s;
                scene.text_font(
                    Point::new(r.right() - OPTIONS_W * s + 6.0 * s, ty),
                    gear.ch().to_string(),
                    self.font_size,
                    p.text_muted,
                    gear.font_id(),
                );
            }
            if let Some(shortcut) = &item.shortcut {
                let w = cx
                    .fonts
                    .measure(shortcut, self.font_size - 1.0, FontId::Ui)
                    .width;
                scene.text(
                    Point::new(right - w, ty),
                    shortcut.clone(),
                    self.font_size - 1.0,
                    p.text_muted,
                );
            }
        }

        scene.end_overlay();
    }

    /// Route an event. Returns the [`Activation`] when an item is clicked.
    ///
    /// Consumes events over the popup (and the dismissing click), so nothing
    /// underneath reacts.
    pub fn event(&mut self, cx: &mut EventCx<'_>, event: &InputEvent) -> Option<Activation> {
        if !self.open {
            return None;
        }
        self.compute(cx.fonts, cx.screen);
        let s = crate::text::scale();

        match event {
            InputEvent::PointerMoved { pos } => {
                self.hovered = self.item_at(*pos);
                if self.panel.contains(*pos) {
                    cx.consume();
                }
            }
            InputEvent::PointerPressed {
                pos,
                button: PointerButton::Primary,
            } => {
                if !self.panel.contains(*pos) {
                    self.close(); // click outside dismisses
                    cx.consume();
                    return None;
                }
                cx.consume();
                let hit = self.item_at(*pos);
                if let Some(index) = hit {
                    let r = self.item_rects[index];
                    let options =
                        self.items[index].has_options && pos.x >= r.right() - OPTIONS_W * s;
                    self.close();
                    return Some(Activation { index, options });
                }
                // A separator or disabled row: swallow, stay open.
            }
            InputEvent::PointerReleased { pos, .. } => {
                if self.panel.contains(*pos) {
                    cx.consume();
                }
            }
            InputEvent::Key {
                key: Key::Escape,
                pressed: true,
                ..
            } => {
                self.close();
                cx.consume();
            }
            _ => {}
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::text::Fonts;
    use crate::theme::Theme;

    #[test]
    fn click_activates_the_item_under_the_pointer() {
        let Some(fonts) = Fonts::load() else {
            return;
        };
        let theme = Theme::dark();
        let screen = Size::new(800.0, 600.0);

        let mut menu = PopupMenu::new();
        menu.open_at(
            Point::new(100.0, 100.0),
            vec![
                MenuItemSpec::new("Rename"),
                MenuItemSpec::separator(),
                MenuItemSpec::new("Delete"),
            ],
        );
        assert!(menu.is_open());
        assert!(crate::popup::is_open());

        let mut cx = EventCx::new(&fonts, &theme, screen);
        // Force geometry, then click the third row ("Delete").
        menu.compute(&fonts, screen);
        let target = menu.item_rects[2].center();

        let got = menu.event(
            &mut cx,
            &InputEvent::PointerPressed {
                pos: target,
                button: PointerButton::Primary,
            },
        );
        assert_eq!(got, Some(Activation { index: 2, options: false }));
        assert!(!menu.is_open(), "activating closes the popup");
        assert!(cx.is_consumed());
    }

    #[test]
    fn opening_near_the_bottom_flips_above_the_anchor() {
        let Some(fonts) = Fonts::load() else {
            return;
        };
        let screen = Size::new(800.0, 600.0);
        let mut menu = PopupMenu::new();
        // Anchor right at the bottom edge: the panel cannot fit below.
        menu.open_at(Point::new(50.0, 590.0), vec![
            MenuItemSpec::new("One"),
            MenuItemSpec::new("Two"),
            MenuItemSpec::new("Three"),
        ]);
        menu.compute(&fonts, screen);

        assert!(
            menu.panel().bottom() <= screen.height + 0.01,
            "popup must stay on screen, got {:?}",
            menu.panel()
        );
        assert!(
            menu.panel().top() < 590.0,
            "popup should flip above the anchor"
        );
    }
}
