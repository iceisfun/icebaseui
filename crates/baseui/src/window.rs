//! Multiple windows.
//!
//! BaseUI drives any number of OS windows from one GPU device and one event
//! loop: a main window plus floating tool windows, dialogs, and (once docking
//! lands) detached dock tabs.
//!
//! Windows cannot be created directly from application code, because that code
//! usually runs deep inside an event handler (a command, a button callback) with
//! no access to the event loop. Instead, requests are **queued** here and drained
//! by the [`App`](crate::App) when the event loop next goes idle:
//!
//! ```no_run
//! use baseui::{window, widget::Label};
//!
//! // From anywhere — a command handler, a button, a script-driven command.
//! window::open(
//!     window::WindowSpec::new("Tool", Label::new("I am a floating window"))
//!         .size(420, 320),
//! );
//! ```
//!
//! Each window owns its own root widget, scene, and pointer state; they share
//! the GPU device, the glyph atlas, the theme, the command registry, and the
//! reactive runtime. A signal write repaints every window.

use std::cell::{Cell, RefCell};

pub use winit::window::WindowId;

use crate::widget::Widget;

/// A request to open a window: its chrome plus the root widget it will show.
pub struct WindowSpec {
    pub(crate) title: String,
    pub(crate) width: u32,
    pub(crate) height: u32,
    pub(crate) position: Option<(i32, i32)>,
    pub(crate) root: Box<dyn Widget>,
}

impl WindowSpec {
    /// A window showing `root`.
    pub fn new(title: impl Into<String>, root: impl Widget + 'static) -> Self {
        WindowSpec {
            title: title.into(),
            width: 480,
            height: 360,
            position: None,
            root: Box::new(root),
        }
    }

    /// Initial size in logical pixels.
    pub fn size(mut self, width: u32, height: u32) -> Self {
        self.width = width;
        self.height = height;
        self
    }

    /// Initial position in physical screen coordinates. A detached dock tab uses
    /// this to open where the pointer let go.
    pub fn position(mut self, x: i32, y: i32) -> Self {
        self.position = Some((x, y));
        self
    }
}

pub(crate) enum Request {
    Open(WindowSpec),
    Close(WindowId),
}

thread_local! {
    static REQUESTS: RefCell<Vec<Request>> = const { RefCell::new(Vec::new()) };
    static DIRTY: Cell<bool> = const { Cell::new(false) };
}

/// Queue a new window. It opens the next time the event loop goes idle.
pub fn open(spec: WindowSpec) {
    REQUESTS.with(|r| r.borrow_mut().push(Request::Open(spec)));
    mark_dirty();
}

/// Queue a window for closing.
pub fn close(id: WindowId) {
    REQUESTS.with(|r| r.borrow_mut().push(Request::Close(id)));
    mark_dirty();
}

pub(crate) fn take_requests() -> Vec<Request> {
    REQUESTS.with(|r| std::mem::take(&mut *r.borrow_mut()))
}

/// Mark the UI dirty; the App repaints **every** window. This is what the
/// reactive change hook calls, since a signal may be read by any window.
pub(crate) fn mark_dirty() {
    DIRTY.with(|d| d.set(true));
}

pub(crate) fn take_dirty() -> bool {
    DIRTY.with(|d| d.replace(false))
}
