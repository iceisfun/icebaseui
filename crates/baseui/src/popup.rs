//! Popup modality.
//!
//! Tracks whether *any* popup (a dropdown menu, a combo box list, …) is
//! currently open. An open popup is modal for the keyboard:
//!
//! - It [`clear`](crate::focus::clear)s keyboard focus when it opens, so a
//!   focused text field stops receiving keystrokes while a menu is down.
//! - The [`App`](crate::App) suppresses global shortcuts while it is open, so a
//!   plain key press doesn't fire a command behind the popup.
//! - Key events still reach the widget tree, so the popup itself can act on them
//!   (e.g. Escape to close).
//!
//! Widgets that own a popup call [`set_open`] when they open/close it.

use std::cell::Cell;

use baseui_core::{Point, Rect, Size};

thread_local! {
    static POPUP_OPEN: Cell<bool> = const { Cell::new(false) };
}

/// Place a popup of `size` relative to `anchor`, kept on screen.
///
/// Prefers directly **below** the anchor. If it would run off the bottom, it
/// **flips above**; if it doesn't fit there either, it is clamped to the bottom
/// edge. Horizontally it is clamped to the screen.
///
/// A context menu passes a zero-size anchor at the click point, so the same
/// function serves dropdowns, combo lists, and right-click menus.
///
/// ```
/// use baseui::popup;
/// use baseui_core::{Rect, Size};
///
/// let screen = Size::new(800.0, 600.0);
/// // Plenty of room below: opens under the anchor.
/// let r = popup::place(Rect::from_xywh(10.0, 100.0, 80.0, 20.0), Size::new(120.0, 90.0), screen, 2.0);
/// assert_eq!(r.top(), 122.0);
///
/// // No room below: flips above the anchor.
/// let r = popup::place(Rect::from_xywh(10.0, 560.0, 80.0, 20.0), Size::new(120.0, 90.0), screen, 2.0);
/// assert_eq!(r.top(), 468.0); // 560 - 2 - 90
/// ```
pub fn place(anchor: Rect, size: Size, screen: Size, margin: f32) -> Rect {
    let below = anchor.bottom() + margin;
    let y = if below + size.height <= screen.height {
        below
    } else {
        let above = anchor.top() - margin - size.height;
        if above >= 0.0 {
            above
        } else {
            // Fits in neither direction: pin to the bottom edge.
            (screen.height - size.height).max(0.0)
        }
    };

    let x = anchor
        .left()
        .min((screen.width - size.width).max(0.0))
        .max(0.0);

    Rect::new(Point::new(x, y), size)
}

/// Mark a popup as opened (`true`) or closed (`false`). Opening also clears
/// keyboard focus, so text fields stop capturing input behind the popup.
pub fn set_open(open: bool) {
    POPUP_OPEN.with(|p| p.set(open));
    if open {
        crate::focus::clear();
    }
}

/// Whether a popup is currently open.
pub fn is_open() -> bool {
    POPUP_OPEN.with(|p| p.get())
}
