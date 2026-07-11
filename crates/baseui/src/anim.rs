//! Animation frames.
//!
//! BaseUI repaints on demand: an input event, or a signal write. That is the
//! right default (an idle UI burns no CPU), but it means a widget that is
//! *animating* has nothing to wake it for the next frame.
//!
//! A widget that needs to keep animating calls [`request_frame`] while painting.
//! The [`App`](crate::App) then schedules another repaint (~60 Hz) instead of
//! going idle, and stops as soon as nobody asks again — so an animation costs
//! frames only while it is actually running.
//!
//! ```ignore
//! // inside a Widget::paint
//! let t = self.started.elapsed().as_secs_f32() / 0.6;
//! if t < 1.0 {
//!     // ...draw the fading highlight...
//!     baseui::anim::request_frame();
//! }
//! ```

use std::cell::Cell;

thread_local! {
    static PENDING: Cell<bool> = const { Cell::new(false) };
}

/// Ask for another frame — call this from `paint` while an animation is running.
pub fn request_frame() {
    PENDING.with(|p| p.set(true));
}

/// Whether a frame was requested (and clear the flag). Called by the `App`.
pub(crate) fn take_pending() -> bool {
    PENDING.with(|p| p.replace(false))
}
