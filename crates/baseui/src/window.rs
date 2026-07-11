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
use std::collections::HashMap;
use std::sync::Arc;

pub use winit::window::WindowId;
use winit::window::Window;

use crate::widget::Widget;

/// A request to open a window: its chrome plus the root widget it will show.
pub struct WindowSpec {
    pub(crate) title: String,
    pub(crate) width: u32,
    pub(crate) height: u32,
    pub(crate) position: Option<(i32, i32)>,
    /// Command context this window activates while focused. Commands registered
    /// with the same context appear in *its* Command Palette (and its shortcuts
    /// fire) but not in other windows'. See [`crate::command`].
    pub(crate) context: Option<String>,
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
            context: None,
            root: Box::new(root),
        }
    }

    /// Scope this window's Command Palette and shortcuts to a context: it shows
    /// global commands **plus** those registered with this context.
    pub fn context(mut self, context: impl Into<String>) -> Self {
        self.context = Some(context.into());
        self
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
    static FOCUSED: Cell<Option<WindowId>> = const { Cell::new(None) };
}

/// The currently focused window, if any. Lets a command act on "this window"
/// (e.g. a context-scoped "Close This Window") without being handed an id.
pub fn focused() -> Option<WindowId> {
    FOCUSED.with(|f| f.get())
}

pub(crate) fn set_focused(id: Option<WindowId>) {
    FOCUSED.with(|f| f.set(id));
}

thread_local! {
    /// Live window handles, so widgets can query/move windows other than their
    /// own — needed for cross-window tab dragging (a torn-off window has to
    /// follow the cursor, and the drop target lives in a different window).
    static HANDLES: RefCell<HashMap<WindowId, Arc<Window>>> = RefCell::new(HashMap::new());
}

pub(crate) fn register_handle(id: WindowId, window: Arc<Window>) {
    HANDLES.with(|h| h.borrow_mut().insert(id, window));
}

pub(crate) fn unregister_handle(id: WindowId) {
    HANDLES.with(|h| h.borrow_mut().remove(&id));
}

fn with_handle<R>(id: WindowId, f: impl FnOnce(&Window) -> R) -> Option<R> {
    HANDLES.with(|h| h.borrow().get(&id).map(|w| f(w)))
}

/// Top-left of a window's **client area** in physical screen coordinates.
pub fn inner_position(id: WindowId) -> Option<(i32, i32)> {
    with_handle(id, |w| w.inner_position().ok().map(|p| (p.x, p.y)))?
}

/// Top-left of a window's outer frame in physical screen coordinates.
pub fn outer_position(id: WindowId) -> Option<(i32, i32)> {
    with_handle(id, |w| w.outer_position().ok().map(|p| (p.x, p.y)))?
}

/// A window's DPI scale factor.
pub fn scale_factor(id: WindowId) -> Option<f64> {
    with_handle(id, |w| w.scale_factor())
}

/// Move a window (its outer frame) to physical screen coordinates.
pub fn set_position(id: WindowId, x: i32, y: i32) {
    with_handle(id, |w| {
        w.set_outer_position(winit::dpi::PhysicalPosition::new(x, y));
    });
}

/// Convert a position inside `id`'s client area (logical px) to physical screen
/// coordinates. This is how a drag gets a *global* cursor position: while a
/// button is held, the pressing window keeps receiving CursorMoved even when the
/// pointer leaves it (the platform's implicit pointer grab), so its local
/// coordinates — possibly negative — still describe where the cursor is.
pub fn to_screen(id: WindowId, local: baseui_core::Point) -> Option<(f32, f32)> {
    let (ox, oy) = inner_position(id)?;
    let scale = scale_factor(id)? as f32;
    Some((ox as f32 + local.x * scale, oy as f32 + local.y * scale))
}

/// Inverse of [`to_screen`]: a physical screen position, in `id`'s logical
/// client coordinates.
pub fn from_screen(id: WindowId, screen: (f32, f32)) -> Option<baseui_core::Point> {
    let (ox, oy) = inner_position(id)?;
    let scale = scale_factor(id)? as f32;
    Some(baseui_core::Point::new(
        (screen.0 - ox as f32) / scale,
        (screen.1 - oy as f32) / scale,
    ))
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
