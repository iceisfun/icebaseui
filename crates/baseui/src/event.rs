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
    /// The main click button — left on a right-handed mouse, right on a
    /// left-handed one; the OS has already applied the user's handedness.
    Primary,
    /// The context-menu button, opposite `Primary` under the same handedness swap.
    Secondary,
    /// The scroll-wheel click.
    Middle,
}

/// Keyboard modifier state.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct Modifiers {
    /// Control is held. Not remapped to Command on macOS — check `meta` for that.
    pub ctrl: bool,
    /// Shift is held. Text input is already case-folded, so this matters only for
    /// shortcuts and range-extending selection.
    pub shift: bool,
    /// Alt / Option is held.
    pub alt: bool,
    /// The "super"/command/Windows key.
    pub meta: bool,
}

impl Modifiers {
    /// True when no modifier is held, i.e. the key press should be taken literally
    /// (typing) rather than as a shortcut.
    pub fn is_empty(self) -> bool {
        !self.ctrl && !self.shift && !self.alt && !self.meta
    }
}

/// A logical key, normalized across platforms.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum Key {
    /// Conventionally cancels/dismisses: closes a popup, reverts an in-progress edit.
    Escape,
    /// Both the main Return key and the numpad Enter; they are not distinguished.
    Enter,
    /// Moves focus. Widgets that want a literal tab character must claim this key,
    /// otherwise focus traversal consumes it.
    Tab,
    /// Deletes backwards from the caret.
    Backspace,
    /// Deletes forwards from the caret.
    Delete,
    /// Left arrow.
    Left,
    /// Right arrow.
    Right,
    /// Up arrow.
    Up,
    /// Down arrow.
    Down,
    /// Home.
    Home,
    /// End.
    End,
    /// Page up.
    PageUp,
    /// Page down.
    PageDown,
    /// The space bar. Delivered as a `Key` *and* as [`InputEvent::Text`], so text
    /// widgets should insert it from the text event only, to avoid a double space.
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
    PointerMoved {
        /// New pointer position, in logical pixels, window-relative.
        pos: Point,
    },
    /// A button was pressed at `pos`.
    PointerPressed {
        /// Pointer position at press time, in logical pixels, window-relative.
        pos: Point,
        /// The button that went down.
        button: PointerButton,
    },
    /// A button was released at `pos`.
    PointerReleased {
        /// Pointer position at release time, in logical pixels, window-relative.
        /// May be outside the widget that saw the press — check before treating a
        /// press/release pair as a click.
        pos: Point,
        /// The button that came up.
        button: PointerButton,
    },
    /// The pointer left the window.
    PointerLeft,
    /// The scroll wheel moved. `delta` is in lines (positive `y` scrolls up /
    /// toward the top); `pos` is the pointer position.
    Scroll {
        /// Pointer position at scroll time, in logical pixels, window-relative;
        /// used to pick the scrollable under the cursor.
        pos: Point,
        /// Scroll amount in lines, not pixels — multiply by a line height to get
        /// a distance. Positive `y` scrolls up / toward the top.
        delta: Vec2,
    },
    /// A key was pressed or released, with the active modifiers.
    Key {
        /// The logical key, after platform normalization.
        key: Key,
        /// `true` on key-down, `false` on key-up. Key repeats arrive as further
        /// key-down events.
        pressed: bool,
        /// Modifiers held at the time of the event.
        mods: Modifiers,
    },
    /// Committed text input (one or more characters), e.g. for text fields.
    Text {
        /// The committed text. More than one character for IME composition and
        /// paste, so insert the whole string rather than assuming a single char.
        text: String,
    },
}
