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

thread_local! {
    static POPUP_OPEN: Cell<bool> = const { Cell::new(false) };
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
