//! [`ScrollArea`] — a vertically scrolling viewport around a taller child.
//!
//! The child is laid out with unbounded height; the ScrollArea clips it to its
//! viewport, offsets it by the scroll position, draws a scrollbar, and routes
//! wheel events. This is what lets [`TreeView`](super::TreeView) and
//! [`PropertyView`](super::PropertyView) show more rows than fit on screen.

use baseui_core::paint::Scene;
use baseui_core::{Rect, Size};

use super::{EventCx, LayoutCx, PaintCx, Widget};
use crate::event::InputEvent;
use crate::layout::Constraints;

/// Logical pixels scrolled per wheel line.
const LINE_PIXELS: f32 = 42.0;
const BAR_WIDTH: f32 = 8.0;

/// A vertical scroll viewport wrapping a single child.
pub struct ScrollArea {
    child: Box<dyn Widget>,
    offset: f32,
    width: Option<f32>,
    height: Option<f32>,
    persist_key: Option<String>,
    /// Cached from the last layout.
    content_height: f32,
    viewport: Size,
}

impl ScrollArea {
    /// Wraps `child`, scrolling it vertically when it is taller than the
    /// viewport.
    pub fn new(child: impl Widget + 'static) -> Self {
        ScrollArea {
            child: Box::new(child),
            offset: 0.0,
            width: None,
            height: None,
            persist_key: None,
            content_height: 0.0,
            viewport: Size::ZERO,
        }
    }

    /// Persist the scroll offset under `key` between runs.
    pub fn persist(mut self, key: impl Into<String>) -> Self {
        self.persist_key = Some(key.into());
        self
    }

    /// Fix the viewport width (otherwise it fills the available width).
    pub fn width(mut self, width: f32) -> Self {
        self.width = Some(width);
        self
    }

    /// Fix the viewport height (otherwise it fills the available height).
    pub fn height(mut self, height: f32) -> Self {
        self.height = Some(height);
        self
    }

    fn max_offset(&self) -> f32 {
        (self.content_height - self.viewport.height).max(0.0)
    }

    /// The child's absolute bounds, translated by the scroll offset.
    fn child_bounds(&self, bounds: Rect) -> Rect {
        Rect::from_xywh(
            bounds.left(),
            bounds.top() - self.offset,
            self.viewport.width,
            self.content_height,
        )
    }
}

impl Widget for ScrollArea {
    fn layout(&mut self, cx: &mut LayoutCx<'_>, constraints: Constraints) -> Size {
        let vw = self
            .width
            .or_else(|| {
                constraints
                    .max
                    .width
                    .is_finite()
                    .then_some(constraints.max.width)
            })
            .unwrap_or(300.0);

        // Lay the child out at the viewport width, unbounded vertically.
        let child_size = self
            .child
            .layout(cx, Constraints::loose(Size::new(vw, f32::INFINITY)));
        self.content_height = child_size.height;

        let vh = self
            .height
            .or_else(|| {
                constraints
                    .max
                    .height
                    .is_finite()
                    .then_some(constraints.max.height)
            })
            .unwrap_or_else(|| child_size.height.min(400.0));

        self.viewport = Size::new(vw, vh);
        self.offset = self.offset.clamp(0.0, self.max_offset());
        constraints.constrain(self.viewport)
    }

    fn paint(&mut self, cx: &mut PaintCx<'_>, bounds: Rect, scene: &mut Scene) {
        scene.push_clip(bounds);
        self.child.paint(cx, self.child_bounds(bounds), scene);
        scene.pop_clip();

        // Scrollbar (visual; wheel-driven).
        let max = self.max_offset();
        if max > 0.0 {
            let p = &cx.theme.palette;
            let track_h = bounds.height();
            let thumb_h = (track_h * (self.viewport.height / self.content_height)).max(24.0);
            let t = self.offset / max;
            let thumb_y = bounds.top() + t * (track_h - thumb_h);
            let bar_w = BAR_WIDTH * crate::text::scale();
            let x = bounds.right() - bar_w - 2.0;
            scene.rounded_rect(
                Rect::from_xywh(x, thumb_y, bar_w, thumb_h),
                p.border,
                bar_w * 0.5,
            );
        }
    }

    fn event(&mut self, cx: &mut EventCx<'_>, bounds: Rect, event: &InputEvent) {
        if let InputEvent::Scroll { pos, delta } = event {
            if bounds.contains(*pos) {
                let step = LINE_PIXELS * crate::text::scale();
                self.offset = (self.offset - delta.y * step).clamp(0.0, self.max_offset());
                return;
            }
        }
        // Route pointer events to the child at its scrolled position, but only
        // when the pointer is over the viewport (so clipped-away rows can't be
        // hit). Exception: while a popup is open, always deliver — a child's
        // popup (drawn in the overlay layer) may extend beyond the viewport and
        // still needs its clicks.
        let deliver = match event {
            InputEvent::PointerMoved { pos }
            | InputEvent::PointerPressed { pos, .. }
            | InputEvent::PointerReleased { pos, .. } => {
                bounds.contains(*pos) || crate::popup::is_open()
            }
            _ => true,
        };
        if deliver {
            self.child.event(cx, self.child_bounds(bounds), event);
        } else if let InputEvent::PointerMoved { .. } = event {
            // Pointer moved off the viewport: let the child clear its hover.
            self.child
                .event(cx, self.child_bounds(bounds), &InputEvent::PointerLeft);
        }
    }

    fn persist_save(&self, store: &mut crate::persist::Store) {
        if let Some(key) = &self.persist_key {
            store.set(key.clone(), &self.offset);
        }
        self.child.persist_save(store);
    }

    fn persist_restore(&mut self, store: &crate::persist::Store) {
        if let Some(key) = &self.persist_key {
            if let Some(offset) = store.get::<f32>(key) {
                self.offset = offset.max(0.0);
            }
        }
        self.child.persist_restore(store);
    }
}
