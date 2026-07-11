//! Keyboard focus.
//!
//! Exactly one widget (or none) holds keyboard focus at a time. Because BaseUI
//! routes input events to the whole tree, a focusable widget claims focus by
//! calling [`set`] with its [`Id`] (typically on click), and decides whether to
//! act on [`Key`](crate::event::Key)/[`Text`](crate::event::InputEvent::Text)
//! events by checking [`has`].
//!
//! Focus lives in thread-local storage, matching the single-UI-thread model of
//! the rest of the framework.

use std::cell::Cell;

use baseui_core::Id;

thread_local! {
    static FOCUS: Cell<Option<Id>> = const { Cell::new(None) };
}

/// Give focus to `id`.
pub fn set(id: Id) {
    FOCUS.with(|f| f.set(Some(id)));
}

/// The currently focused id, if any.
pub fn current() -> Option<Id> {
    FOCUS.with(|f| f.get())
}

/// Whether `id` currently holds focus.
pub fn has(id: Id) -> bool {
    FOCUS.with(|f| f.get() == Some(id))
}

/// Clear keyboard focus.
pub fn clear() {
    FOCUS.with(|f| f.set(None));
}
