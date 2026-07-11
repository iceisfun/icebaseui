//! System clipboard access (text).
//!
//! A thin, failure-tolerant wrapper over `arboard`. The clipboard handle is
//! created lazily and cached in thread-local storage; all errors (no display,
//! unavailable clipboard, etc.) degrade to `None`/no-op so text editing keeps
//! working without a clipboard.

use std::cell::RefCell;

thread_local! {
    static CLIPBOARD: RefCell<Option<arboard::Clipboard>> = const { RefCell::new(None) };
}

fn with_clipboard<R>(f: impl FnOnce(&mut arboard::Clipboard) -> R) -> Option<R> {
    CLIPBOARD.with(|c| {
        let mut slot = c.borrow_mut();
        if slot.is_none() {
            *slot = arboard::Clipboard::new().ok();
        }
        slot.as_mut().map(f)
    })
}

/// Read UTF-8 text from the clipboard, or `None` if empty/unavailable.
pub fn get_text() -> Option<String> {
    with_clipboard(|c| c.get_text().ok()).flatten()
}

/// Write UTF-8 text to the clipboard (best-effort).
pub fn set_text(text: &str) {
    let _ = with_clipboard(|c| c.set_text(text.to_string()));
}
