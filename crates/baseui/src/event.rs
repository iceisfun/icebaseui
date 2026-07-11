//! Input events delivered to the widget tree.
//!
//! These are the framework's normalized, backend-independent input events —
//! translated from raw winit events by the [`App`](crate::App) shell, with all
//! positions in **logical** pixels. This is distinct from the future
//! application-level *event bus* (SelectionChanged, DocumentOpened, …); this
//! module is only about raw pointer/keyboard input routed to widgets.

use baseui_core::Point;

/// A pointer (mouse) button.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum PointerButton {
    Primary,
    Secondary,
    Middle,
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
}
