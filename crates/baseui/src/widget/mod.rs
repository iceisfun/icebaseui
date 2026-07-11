//! The widget system: the retained tree of UI objects.
//!
//! A [`Widget`] persists across frames (it is *retained*) and holds its own
//! state. Each frame the framework runs three passes over the tree:
//!
//! 1. [`Widget::layout`] — the parent hands down [`Constraints`], the widget
//!    returns the [`Size`] it chose.
//! 2. [`Widget::paint`] — the widget emits primitives into the [`Scene`], given
//!    the absolute `bounds` its parent assigned.
//! 3. [`Widget::event`] — raw [`InputEvent`]s are routed down with the same
//!    `bounds`, letting the widget hit-test itself.
//!
//! Widgets bind to application state through the reactive [`Signal`]s in
//! `baseui-core`: a [`Label`] reads a signal in its content closure, a
//! [`Button`] writes one in its click handler, and
//! [`set_on_change`](baseui_core::reactive::set_on_change) turns any signal
//! write into a repaint. That is the *reactive* half of "retained + reactive".
//!
//! [`Constraints`]: crate::layout::Constraints
//! [`Signal`]: baseui_core::Signal

mod button;
mod checkbox;
mod combobox;
mod dock;
mod drag_value;
mod hex_view;
mod label;
mod menu;
mod popup_menu;
mod property;
mod scroll;
mod slider;
mod split;
mod stack;
pub mod statusbar;
mod tabs;
mod textarea;
mod textbox;
mod toolbar;
mod tree;

pub use button::Button;
pub use checkbox::Checkbox;
pub use combobox::ComboBox;
pub use dock::{DockArea, DockAxis, DockNode, Panel};
pub use drag_value::DragValue;
pub use hex_view::HexView;
pub use label::Label;
pub use menu::{Menu, MenuBar};
pub use popup_menu::{Activation, MenuItemSpec, PopupMenu};
pub use property::{PropGroup, PropertyView};
pub use scroll::ScrollArea;
pub use slider::Slider;
pub use split::Split;
pub use stack::{Column, Row};
pub use statusbar::{StatusBar, StatusItem};
pub use tabs::{TabStrip, TabView};
pub use textarea::{Checker, DEFAULT_LINE_SPACING, Diagnostic, Highlighter, Span, TextArea};
pub use textbox::TextBox;
pub use toolbar::Toolbar;
pub use tree::{TreeNode, TreeView};

use baseui_core::Size;
use baseui_core::paint::Scene;
use baseui_core::{Point, Rect};

use crate::event::InputEvent;
use crate::layout::Constraints;
use crate::text::Fonts;
use crate::theme::Theme;

/// Context shared by the layout pass.
pub struct LayoutCx<'a> {
    /// Font metrics — measuring text is most of what layout does.
    pub fonts: &'a Fonts,
    /// Metrics and colours; consulted here for the sizes baked into the theme.
    pub theme: &'a Theme,
    /// Which window is being laid out, when known.
    pub window: Option<crate::window::WindowId>,
}

/// Context shared by the paint pass.
pub struct PaintCx<'a> {
    /// Font metrics, for positioning glyphs within the rect layout settled on.
    pub fonts: &'a Fonts,
    /// The colours to draw with.
    pub theme: &'a Theme,
    /// The window's logical size — popups use it to stay on screen.
    pub screen: Size,
    /// Which window is being painted, when known.
    pub window: Option<crate::window::WindowId>,
}

/// Context shared by the event pass.
///
/// Carries a `consumed` flag: a widget that handles an event (e.g. an open menu
/// swallowing clicks over its dropdown) calls [`EventCx::consume`], and
/// containers stop delivering that event to later siblings — instead sending
/// them a synthetic [`InputEvent::PointerLeft`] so their hover state clears.
/// This is what stops clicks/hover from bleeding through a popup to the widgets
/// beneath it.
pub struct EventCx<'a> {
    /// Font metrics — hit-testing text (a caret from a click x) needs them.
    pub fonts: &'a Fonts,
    /// Metrics and colours, for widgets whose hit areas follow theme sizes.
    pub theme: &'a Theme,
    /// The window's logical size — popups use it to stay on screen.
    pub screen: Size,
    /// Which window this event is being routed in. A widget needs this to close
    /// or re-parent its own window (a detached dock panel docking itself back).
    pub window: Option<crate::window::WindowId>,
    consumed: bool,
}

impl<'a> EventCx<'a> {
    /// A context for one event, not yet consumed and not yet tagged with a
    /// window — see [`EventCx::with_window`].
    pub fn new(fonts: &'a Fonts, theme: &'a Theme, screen: Size) -> Self {
        EventCx {
            fonts,
            theme,
            screen,
            window: None,
            consumed: false,
        }
    }

    /// Tag this context with the window it belongs to.
    pub fn with_window(mut self, id: crate::window::WindowId) -> Self {
        self.window = Some(id);
        self
    }

    /// Mark the current event as handled; later siblings will not receive it.
    pub fn consume(&mut self) {
        self.consumed = true;
    }

    /// Whether the current event has already been consumed upstream.
    pub fn is_consumed(&self) -> bool {
        self.consumed
    }

    /// The event a child should receive given the current consumed state: the
    /// real `event` if nothing has consumed it yet, otherwise a synthetic
    /// [`InputEvent::PointerLeft`] so the child clears any hover state.
    pub(crate) fn effective<'e>(&self, event: &'e InputEvent) -> &'e InputEvent {
        const LEAVE: InputEvent = InputEvent::PointerLeft;
        if self.consumed { &LEAVE } else { event }
    }
}

/// A retained UI element. See the [module docs](self) for the three-pass model.
pub trait Widget {
    /// Choose a size within `constraints`. Containers should also record the
    /// positions/sizes of their children here for use in `paint`/`event`.
    fn layout(&mut self, cx: &mut LayoutCx<'_>, constraints: Constraints) -> Size;

    /// Emit primitives for this widget, occupying the absolute `bounds` the
    /// parent assigned (its origin is this widget's top-left on screen).
    fn paint(&mut self, cx: &mut PaintCx<'_>, bounds: Rect, scene: &mut Scene);

    /// Handle a routed input event. `bounds` is this widget's absolute rect, so
    /// the widget can hit-test the pointer position itself. Default: ignore.
    fn event(&mut self, cx: &mut EventCx<'_>, bounds: Rect, event: &InputEvent) {
        let _ = (cx, bounds, event);
    }

    /// Write any persistable state into `store`. Containers should forward to
    /// their children. Default: nothing to persist.
    fn persist_save(&self, store: &mut crate::persist::Store) {
        let _ = store;
    }

    /// Restore persistable state from `store` (called once before the first
    /// layout). Containers should forward to their children. Default: no-op.
    fn persist_restore(&mut self, store: &crate::persist::Store) {
        let _ = store;
    }

    /// Box this widget — sugar for building trees.
    fn boxed(self) -> Box<dyn Widget>
    where
        Self: Sized + 'static,
    {
        Box::new(self)
    }
}

/// Translate a child rect stored relative to a container's origin into an
/// absolute rect on screen.
pub(crate) fn absolute(container: Rect, child_rel: Rect) -> Rect {
    Rect::new(
        Point::new(
            container.left() + child_rel.left(),
            container.top() + child_rel.top(),
        ),
        child_rel.size,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event::{InputEvent, PointerButton};
    use crate::text::Fonts;
    use crate::theme::Theme;
    use baseui_core::Insets;
    use std::cell::Cell;
    use std::rc::Rc;

    /// A widget of fixed size that records the absolute bounds it is handed
    /// during `event` — used to assert container layout positions.
    struct Probe {
        size: Size,
        seen: Rc<Cell<Option<Rect>>>,
    }

    impl Widget for Probe {
        fn layout(&mut self, _cx: &mut LayoutCx<'_>, c: Constraints) -> Size {
            c.constrain(self.size)
        }
        fn paint(&mut self, _cx: &mut PaintCx<'_>, _bounds: Rect, _scene: &mut Scene) {}
        fn event(&mut self, _cx: &mut EventCx<'_>, bounds: Rect, _event: &InputEvent) {
            self.seen.set(Some(bounds));
        }
    }

    #[test]
    fn column_positions_children_with_padding_and_spacing() {
        let Some(fonts) = Fonts::load() else {
            eprintln!("no system fonts; skipping");
            return;
        };
        let theme = Theme::dark();

        let seen0 = Rc::new(Cell::new(None));
        let seen1 = Rc::new(Cell::new(None));
        let mut col = Column::new()
            .padding(Insets::all(10.0))
            .spacing(5.0)
            .child(Probe {
                size: Size::new(100.0, 20.0),
                seen: seen0.clone(),
            })
            .child(Probe {
                size: Size::new(100.0, 30.0),
                seen: seen1.clone(),
            });

        let mut lcx = LayoutCx {
            fonts: &fonts,
            theme: &theme,
            window: None,
        };
        let size = col.layout(&mut lcx, Constraints::loose(Size::new(1000.0, 1000.0)));
        // 10 pad + 20 + 5 spacing + 30 + 10 pad = 75 tall; 100 + 20 pad = 120 wide.
        assert_eq!(size, Size::new(120.0, 75.0));

        let mut ecx = EventCx::new(&fonts, &theme, Size::new(1000.0, 1000.0));
        col.event(
            &mut ecx,
            Rect::new(Point::ZERO, size),
            &InputEvent::PointerMoved { pos: Point::ZERO },
        );
        assert_eq!(seen0.get(), Some(Rect::from_xywh(10.0, 10.0, 100.0, 20.0)));
        assert_eq!(seen1.get(), Some(Rect::from_xywh(10.0, 35.0, 100.0, 30.0)));
    }

    #[test]
    fn button_click_fires_on_press_then_release_inside() {
        let Some(fonts) = Fonts::load() else {
            eprintln!("no system fonts; skipping");
            return;
        };
        let theme = Theme::dark();

        let clicks = Rc::new(Cell::new(0));
        let c2 = clicks.clone();
        let mut button = Button::new("Go").on_click(move || c2.set(c2.get() + 1));

        let mut lcx = LayoutCx {
            fonts: &fonts,
            theme: &theme,
            window: None,
        };
        let size = button.layout(&mut lcx, Constraints::loose(Size::new(1000.0, 1000.0)));
        let bounds = Rect::new(Point::ZERO, size);
        let inside = bounds.center();
        let outside = Point::new(bounds.right() + 50.0, bounds.bottom() + 50.0);

        let mut ecx = EventCx::new(&fonts, &theme, Size::new(1000.0, 1000.0));
        let mut send = |b: &mut Button, e: InputEvent| {
            b.event(&mut ecx, bounds, &e);
        };

        // Press + release inside => one click.
        send(
            &mut button,
            InputEvent::PointerPressed {
                pos: inside,
                button: PointerButton::Primary,
            },
        );
        send(
            &mut button,
            InputEvent::PointerReleased {
                pos: inside,
                button: PointerButton::Primary,
            },
        );
        assert_eq!(clicks.get(), 1);

        // Press inside, release outside => no additional click.
        send(
            &mut button,
            InputEvent::PointerPressed {
                pos: inside,
                button: PointerButton::Primary,
            },
        );
        send(
            &mut button,
            InputEvent::PointerReleased {
                pos: outside,
                button: PointerButton::Primary,
            },
        );
        assert_eq!(clicks.get(), 1);

        // A press entirely outside never arms the button.
        send(
            &mut button,
            InputEvent::PointerPressed {
                pos: outside,
                button: PointerButton::Primary,
            },
        );
        send(
            &mut button,
            InputEvent::PointerReleased {
                pos: inside,
                button: PointerButton::Primary,
            },
        );
        assert_eq!(clicks.get(), 1);
    }
}
