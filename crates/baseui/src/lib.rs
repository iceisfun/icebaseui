//! # BaseUI
//!
//! A modern, extensible desktop UI framework for engineering applications —
//! IDEs, debuggers, visualization tools, editors, CAD software, and custom
//! internal apps.
//!
//! Built on **winit + wgpu**, with a *retained + reactive* architecture: the
//! widget tree is retained across frames, and state lives in
//! [`baseui_core::reactive`] signals whose writes mark the affected
//! windows for repaint.
//!
//! ```no_run
//! use baseui::widget::{Button, Column, Label};
//! use baseui::{App, core::create_signal};
//!
//! let count = create_signal(0);
//! let root = Column::new()
//!     .child(Label::dynamic(move || format!("count: {}", count.get())))
//!     .child(Button::new("increment").on_click(move || count.set(count.get() + 1)));
//!
//! App::new().with_title("Counter").with_root(root).run().unwrap();
//! ```
//!
//! # The shape of it
//!
//! - [`app`] — the [`App`] shell: many windows on one GPU device and one event
//!   loop, with pointer/keyboard routing, persistence, and the command palette.
//! - [`widget`] — the [`Widget`] trait (three passes: `layout`, `paint`, `event`)
//!   and the widget set: `Label`, `Button`, `TextBox`, `TextArea`, `TreeView`,
//!   `PropertyView`, `HexView`, `DockArea`, and the rest.
//! - [`layout`] — Flutter-style box constraints.
//! - [`text`] — fonts and **text measurement**: metrics, carets, hit-testing,
//!   truncation, wrapping. See `docs/text.md`.
//! - [`render`] — the wgpu backend: one instanced-quad pipeline draws every
//!   primitive (SDF rounded rects, glyphs from a shared atlas, squiggles).
//! - [`theme`] — design tokens: palette, spacing, radius, type, motion.
//! - [`command`] — the command registry: the **single source of truth** for
//!   menus, the toolbar, shortcuts, and the Command Palette (F1).
//! - [`undo`] — the [`History`](undo::History) trait and a built-in undo stack.
//! - [`bus`], [`persist`], [`icon`], [`focus`], [`popup`], [`window`], [`anim`] —
//!   the systems layer.
//!
//! Scripting lives in the optional `baseui-lua` crate: Lua registers commands,
//! shortcuts, events, and status items, and can measure text. It deliberately
//! cannot implement widgets — those are Rust. See `docs/scripting.md`.
//!
//! # Writing a widget
//!
//! Implement three methods. Nothing is hidden from you and nothing is magic:
//!
//! ```
//! use baseui::layout::Constraints;
//! use baseui::paint::Scene;
//! use baseui::widget::{EventCx, LayoutCx, PaintCx, Widget};
//! use baseui::{Rect, Size};
//! use baseui::event::InputEvent;
//!
//! struct Dot;
//!
//! impl Widget for Dot {
//!     fn layout(&mut self, _cx: &mut LayoutCx<'_>, c: Constraints) -> Size {
//!         c.constrain(Size::new(16.0, 16.0))
//!     }
//!
//!     fn paint(&mut self, cx: &mut PaintCx<'_>, bounds: Rect, scene: &mut Scene) {
//!         scene.rounded_rect(bounds, cx.theme.palette.accent, 8.0);
//!     }
//!
//!     fn event(&mut self, _cx: &mut EventCx<'_>, _bounds: Rect, _event: &InputEvent) {}
//! }
//! ```
//!
//! Three rules the framework will not enforce for you:
//!
//! 1. **Measure with [`text::Fonts`]**, never by re-deriving glyph advances. It is
//!    the same call the renderer steps its pen by, so anything you compute from it
//!    lands exactly on the drawn glyphs.
//! 2. **Containers forward events to children *before* interpreting them as their
//!    own chrome**, and check [`EventCx::is_consumed`](widget::EventCx::is_consumed)
//!    before acting. Otherwise clicks fall through open popups onto whatever is
//!    behind them.
//! 3. **Global (non-signal) state must call [`window::mark_dirty`]** when it
//!    changes. Signals repaint automatically; a plain `static` does not.
//!
//! Logical (DPI-independent) pixels throughout, origin top-left, y down.

// Every public item carries documentation. This is a lint, not a convention,
// because a convention is what let 213 items go undocumented in the first place.
#![warn(missing_docs)]

pub mod anim;
pub mod app;
pub mod bus;
pub mod clipboard;
pub mod command;
pub mod event;
pub mod focus;
pub mod icon;
pub mod layout;
pub mod persist;
pub mod popup;
pub mod render;
pub mod text;
pub mod theme;
pub mod undo;
pub mod widget;
pub mod window;

pub use app::{App, Frame, WindowConfig};
pub use icon::Icon;
pub use theme::Theme;
pub use widget::{
    Button, Checkbox, Column, ComboBox, DragValue, HexView, Label, Menu, MenuBar, PropGroup,
    PropertyView, Row, ScrollArea, Slider, Split, StatusBar, StatusItem, TabView, TextBox, Toolbar,
    TreeNode, TreeView, Widget,
};

// Re-export the dependency-free core so downstream code has one import root.
pub use baseui_core as core;
pub use baseui_core::paint::{self, Scene};
pub use baseui_core::reactive;
pub use baseui_core::{Color, Id, Insets, Point, Rect, Signal, Size, Vec2};
