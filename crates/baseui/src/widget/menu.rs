//! [`MenuBar`] and [`Menu`] — a top menu bar with dropdown menus.
//!
//! The dropdown itself is a [`PopupMenu`], shared with the combo box and context
//! menus: it handles placement (staying on screen), overlay painting, event
//! consumption, and keyboard modality. `MenuBar` only owns the titles and maps
//! an [`Activation`](super::Activation) back to the item's handler.

use baseui_core::paint::Scene;
use baseui_core::{Point, Rect, Size};

use super::{EventCx, LayoutCx, MenuItemSpec, PaintCx, PopupMenu, Widget};
use crate::event::{InputEvent, PointerButton};
use crate::icon::Icon;
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

    /// The popup specs for this menu's entries (indices align with `entries`).
    fn specs(&self) -> Vec<MenuItemSpec> {
        self.entries
            .iter()
            .map(|entry| match entry {
                Entry::Separator => MenuItemSpec::separator(),
                Entry::Item {
                    icon,
                    label,
                    options,
                    ..
                } => {
                    let mut spec = MenuItemSpec::new(label.clone());
                    if let Some(icon) = icon {
                        spec = spec.icon(*icon);
                    }
                    if options.is_some() {
                        spec = spec.with_options();
                    }
                    spec
                }
            })
            .collect()
    }
}

/// The top menu bar.
pub struct MenuBar {
    menus: Vec<Menu>,
    /// Which menu's dropdown is showing.
    open: Option<usize>,
    hovered_title: Option<usize>,
    font_size: f32,
    bar_h: f32,
    title_rects: Vec<Rect>,
    popup: PopupMenu,
}

impl MenuBar {
    pub fn new() -> Self {
        MenuBar {
            menus: Vec::new(),
            open: None,
            hovered_title: None,
            font_size: 14.0,
            bar_h: 30.0,
            title_rects: Vec::new(),
            popup: PopupMenu::new(),
        }
    }

    pub fn menu(mut self, menu: Menu) -> Self {
        self.menus.push(menu);
        self
    }

    fn open_menu(&mut self, index: usize, bounds: Rect) {
        let anchor = super::absolute(bounds, self.title_rects[index]);
        self.popup.open_below(anchor, self.menus[index].specs());
        self.open = Some(index);
    }

    fn close_menu(&mut self) {
        self.popup.close();
        self.open = None;
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
                Point::new(
                    tr.left() + cx.theme.spacing.md,
                    tr.top() + (tr.height() - line_h) * 0.5,
                ),
                menu.title.clone(),
                self.font_size,
                p.text,
            );
        }

        self.popup.paint(cx, scene);
    }

    fn event(&mut self, cx: &mut EventCx<'_>, bounds: Rect, event: &InputEvent) {
        // A click on a title toggles that menu, and must not be read by the open
        // popup as a dismissing click.
        if let InputEvent::PointerPressed {
            pos,
            button: PointerButton::Primary,
        } = event
        {
            if let Some(i) = self
                .title_rects
                .iter()
                .position(|r| super::absolute(bounds, *r).contains(*pos))
            {
                if self.open == Some(i) {
                    self.close_menu();
                } else {
                    self.open_menu(i, bounds);
                }
                cx.consume();
                return;
            }
        }

        if let Some(activation) = self.popup.event(cx, event) {
            if let Some(menu_index) = self.open {
                if let Some(Entry::Item { on, options, .. }) =
                    self.menus[menu_index].entries.get_mut(activation.index)
                {
                    match (activation.options, options) {
                        (true, Some(opts)) => opts(),
                        _ => on(),
                    }
                }
            }
            self.open = None;
            return;
        }
        // The popup may have dismissed itself (click outside, Escape).
        if !self.popup.is_open() {
            self.open = None;
        }
        if cx.is_consumed() {
            self.hovered_title = None;
            return;
        }

        match event {
            InputEvent::PointerMoved { pos } => {
                self.hovered_title = self
                    .title_rects
                    .iter()
                    .position(|r| super::absolute(bounds, *r).contains(*pos));
            }
            InputEvent::PointerLeft => self.hovered_title = None,
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::text::Fonts;
    use crate::theme::Theme;
    use std::cell::Cell;
    use std::rc::Rc;

    /// Regression for the PopupMenu refactor: opening a menu and clicking an
    /// item must still run that item's handler.
    #[test]
    fn clicking_a_menu_item_runs_its_handler() {
        let Some(fonts) = Fonts::load() else {
            return;
        };
        let theme = Theme::dark();
        let screen = Size::new(800.0, 600.0);

        let ran = Rc::new(Cell::new(0));
        let r2 = ran.clone();
        let mut bar = MenuBar::new().menu(
            Menu::new("File")
                .item("New", move || r2.set(r2.get() + 1))
                .separator()
                .item("Quit", || {}),
        );

        let mut lcx = LayoutCx {
            fonts: &fonts,
            theme: &theme,
            window: None,
        };
        let size = bar.layout(&mut lcx, Constraints::loose(Size::new(800.0, 40.0)));
        let bounds = Rect::new(Point::ZERO, size);

        // Click the "File" title -> the dropdown opens.
        let title = super::super::absolute(bounds, bar.title_rects[0]);
        let mut cx = EventCx::new(&fonts, &theme, screen);
        bar.event(
            &mut cx,
            bounds,
            &InputEvent::PointerPressed {
                pos: title.center(),
                button: PointerButton::Primary,
            },
        );
        assert!(bar.popup.is_open());

        // A move lays the popup out, so we can find its panel.
        let mut cx = EventCx::new(&fonts, &theme, screen);
        bar.event(
            &mut cx,
            bounds,
            &InputEvent::PointerMoved {
                pos: title.center(),
            },
        );
        let panel = bar.popup.panel();
        assert!(panel.height() > 0.0);

        // Click the first item ("New").
        let mut cx = EventCx::new(&fonts, &theme, screen);
        bar.event(
            &mut cx,
            bounds,
            &InputEvent::PointerPressed {
                pos: Point::new(panel.left() + 10.0, panel.top() + 6.0),
                button: PointerButton::Primary,
            },
        );
        assert_eq!(ran.get(), 1, "menu item handler should have run");
        assert!(!bar.popup.is_open(), "activating an item closes the menu");
    }
}
