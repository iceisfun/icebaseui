//! Input events delivered to the widget tree.
//!
//! These are the framework's normalized, backend-independent input events —
//! translated from raw winit events by the [`App`](crate::App) shell, with all
//! positions in **logical** pixels. This is distinct from the future
//! application-level *event bus* (SelectionChanged, DocumentOpened, …); this
//! module is only about raw pointer/keyboard input routed to widgets.

use baseui_core::{Point, Vec2};

/// A pointer (mouse) button.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum PointerButton {
    Primary,
    Secondary,
    Middle,
}

/// Keyboard modifier state.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct Modifiers {
    pub ctrl: bool,
    pub shift: bool,
    pub alt: bool,
    /// The "super"/command/Windows key.
    pub meta: bool,
}

impl Modifiers {
    pub fn is_empty(self) -> bool {
        !self.ctrl && !self.shift && !self.alt && !self.meta
    }
}

/// A logical key, normalized across platforms.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum Key {
    Escape,
    Enter,
    Tab,
    Backspace,
    Delete,
    Left,
    Right,
    Up,
    Down,
    Home,
    End,
    PageUp,
    PageDown,
    Space,
    /// A function key, 1-based (`Function(1)` == F1).
    Function(u8),
    /// A printable character key (already case-folded by the platform).
    Character(char),
    /// Any other named key, keyed by its platform name.
    Named(String),
}

/// A raw input event, positions in logical pixels.
#[derive(Clone, Debug)]
pub enum InputEvent {
    /// The pointer moved to `pos`.
    PointerMoved { pos: Point },
    /// A button was pressed at `pos`.
    PointerPressed { pos: Point, button: PointerButton },
    /// A button was released at `pos`.
    PointerReleased { pos: Point, button: PointerButton },
    /// The pointer left the window.
    PointerLeft,
    /// The scroll wheel moved. `delta` is in lines (positive `y` scrolls up /
    /// toward the top); `pos` is the pointer position.
    Scroll { pos: Point, delta: Vec2 },
    /// A key was pressed or released, with the active modifiers.
    Key {
        key: Key,
        pressed: bool,
        mods: Modifiers,
    },
    /// Committed text input (one or more characters), e.g. for text fields.
    Text { text: String },
}
