//! # BaseUI
//!
//! A modern, extensible desktop UI framework for engineering applications —
//! IDEs, debuggers, visualization tools, editors, CAD software, and custom
//! internal apps.
//!
//! BaseUI is built on **winit + wgpu** and follows a *retained + reactive*
//! architecture: UI state lives in [`reactive`] signals, and the widget tree
//! updates the parts of the screen that actually changed.
//!
//! This is the **foundation milestone**. Today it provides:
//!
//! - the [`App`] shell (a themed window on a winit event loop),
//! - the wgpu [`render`]er (currently a themed clear),
//! - the [`Theme`] engine (design tokens: colors, spacing, radius, type, motion),
//! - and, via [`baseui_core`], the geometry, color, id, and reactive primitives.
//!
//! ```no_run
//! use baseui::{App, Theme};
//!
//! fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     App::new()
//!         .with_title("Hello BaseUI")
//!         .with_theme(Theme::dark())
//!         .run()
//! }
//! ```

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
