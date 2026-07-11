//! [`Column`] and [`Row`] — linear stacking containers.

use baseui_core::paint::Scene;
use baseui_core::{Insets, Point, Rect, Size};

use super::{EventCx, LayoutCx, PaintCx, Widget, absolute};
use crate::event::InputEvent;
use crate::layout::Constraints;

/// Shared state/behavior for a linear container along one axis.
struct Stack {
    children: Vec<Box<dyn Widget>>,
    spacing: f32,
    padding: Insets,
    /// Child rects relative to this container's origin, filled during layout.
    child_rects: Vec<Rect>,
}

impl Stack {
    fn new() -> Self {
        Stack {
            children: Vec::new(),
            spacing: 0.0,
            padding: Insets::ZERO,
            child_rects: Vec::new(),
        }
    }

    fn layout(&mut self, cx: &mut LayoutCx<'_>, constraints: Constraints, vertical: bool) -> Size {
        self.child_rects.clear();

        let avail_w = (constraints.max.width - self.padding.horizontal()).max(0.0);
        let avail_h = (constraints.max.height - self.padding.vertical()).max(0.0);

        // Main-axis cursor and cross-axis extent.
        let mut cursor = if vertical { self.padding.top } else { self.padding.left };
        let mut cross_max = 0.0f32;
        let count = self.children.len();

        for (i, child) in self.children.iter_mut().enumerate() {
            let child_constraints = if vertical {
                Constraints::loose_width(avail_w)
            } else {
                Constraints::loose_height(avail_h)
            };
            let size = child.layout(cx, child_constraints);

            let rect = if vertical {
                Rect::new(Point::new(self.padding.left, cursor), size)
            } else {
                Rect::new(Point::new(cursor, self.padding.top), size)
            };
            self.child_rects.push(rect);

            if vertical {
                cursor += size.height;
                cross_max = cross_max.max(size.width);
            } else {
                cursor += size.width;
                cross_max = cross_max.max(size.height);
            }
            if i + 1 < count {
                cursor += self.spacing;
            }
        }

        let size = if vertical {
            Size::new(
                cross_max + self.padding.horizontal(),
                cursor + self.padding.bottom,
            )
        } else {
            Size::new(
                cursor + self.padding.right,
                cross_max + self.padding.vertical(),
            )
        };
        constraints.constrain(size)
    }

    fn paint(&mut self, cx: &mut PaintCx<'_>, bounds: Rect, scene: &mut Scene) {
        for (child, rel) in self.children.iter_mut().zip(&self.child_rects) {
            child.paint(cx, absolute(bounds, *rel), scene);
        }
    }

    fn event(&mut self, cx: &mut EventCx<'_>, bounds: Rect, event: &InputEvent) {
        for (child, rel) in self.children.iter_mut().zip(&self.child_rects) {
            let ev = cx.effective(event);
            child.event(cx, absolute(bounds, *rel), ev);
        }
    }

    fn persist_save(&self, store: &mut crate::persist::Store) {
        for child in &self.children {
            child.persist_save(store);
        }
    }

    fn persist_restore(&mut self, store: &crate::persist::Store) {
        for child in &mut self.children {
            child.persist_restore(store);
        }
    }
}

/// A vertical stack of widgets, top to bottom.
pub struct Column(Stack);

impl Column {
    pub fn new() -> Self {
        Column(Stack::new())
    }

    /// Add a child.
    pub fn child(mut self, widget: impl Widget + 'static) -> Self {
        self.0.children.push(Box::new(widget));
        self
    }

    /// Add an already-boxed child.
    pub fn child_boxed(mut self, widget: Box<dyn Widget>) -> Self {
        self.0.children.push(widget);
        self
    }

    /// Gap between children in logical pixels.
    pub fn spacing(mut self, spacing: f32) -> Self {
        self.0.spacing = spacing;
        self
    }

    /// Padding around the children.
    pub fn padding(mut self, padding: Insets) -> Self {
        self.0.padding = padding;
        self
    }
}

impl Default for Column {
    fn default() -> Self {
        Column::new()
    }
}

impl Widget for Column {
    fn layout(&mut self, cx: &mut LayoutCx<'_>, constraints: Constraints) -> Size {
        self.0.layout(cx, constraints, true)
    }
    fn paint(&mut self, cx: &mut PaintCx<'_>, bounds: Rect, scene: &mut Scene) {
        self.0.paint(cx, bounds, scene);
    }
    fn event(&mut self, cx: &mut EventCx<'_>, bounds: Rect, event: &InputEvent) {
        self.0.event(cx, bounds, event);
    }
    fn persist_save(&self, store: &mut crate::persist::Store) {
        self.0.persist_save(store);
    }
    fn persist_restore(&mut self, store: &crate::persist::Store) {
        self.0.persist_restore(store);
    }
}

/// A horizontal stack of widgets, left to right.
pub struct Row(Stack);

impl Row {
    pub fn new() -> Self {
        Row(Stack::new())
    }

    pub fn child(mut self, widget: impl Widget + 'static) -> Self {
        self.0.children.push(Box::new(widget));
        self
    }

    pub fn child_boxed(mut self, widget: Box<dyn Widget>) -> Self {
        self.0.children.push(widget);
        self
    }

    pub fn spacing(mut self, spacing: f32) -> Self {
        self.0.spacing = spacing;
        self
    }

    pub fn padding(mut self, padding: Insets) -> Self {
        self.0.padding = padding;
        self
    }
}

impl Default for Row {
    fn default() -> Self {
        Row::new()
    }
}

impl Widget for Row {
    fn layout(&mut self, cx: &mut LayoutCx<'_>, constraints: Constraints) -> Size {
        self.0.layout(cx, constraints, false)
    }
    fn paint(&mut self, cx: &mut PaintCx<'_>, bounds: Rect, scene: &mut Scene) {
        self.0.paint(cx, bounds, scene);
    }
    fn event(&mut self, cx: &mut EventCx<'_>, bounds: Rect, event: &InputEvent) {
        self.0.event(cx, bounds, event);
    }
    fn persist_save(&self, store: &mut crate::persist::Store) {
        self.0.persist_save(store);
    }
    fn persist_restore(&mut self, store: &crate::persist::Store) {
        self.0.persist_restore(store);
    }
}
