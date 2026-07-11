//! [`Split`] — a horizontal row of panes separated by draggable gutters.
//!
//! Panes are either **fixed-width** (resizable by dragging their gutter) or
//! **flexible** (they absorb the remaining width, so the middle content grows as
//! the window widens). This is the resizable app-frame layout: e.g. a fixed
//! outliner on the left, a flexible content area in the middle, and a fixed
//! inspector on the right, with drag bars between them.
//!
//! Only horizontal splitting is implemented today; vertical and recursive
//! nesting (per the SOW) compose the same way and can be added without changing
//! this API.

use baseui_core::paint::Scene;
use baseui_core::{Rect, Size};

use super::{EventCx, LayoutCx, PaintCx, Widget, absolute};
use crate::event::{InputEvent, PointerButton};
use crate::layout::Constraints;

const DEFAULT_GUTTER: f32 = 6.0;
const DEFAULT_MIN: f32 = 120.0;
const DEFAULT_MAX: f32 = 900.0;

enum Mode {
    Fixed(f32),
    Flex,
}

struct Pane {
    widget: Box<dyn Widget>,
    mode: Mode,
    min: f32,
    max: f32,
    rect: Rect,
}

struct Drag {
    /// Index of the fixed pane whose width the gutter controls.
    pane: usize,
    /// +1 if the controlled pane is left of the gutter, -1 if right.
    sign: f32,
    start_x: f32,
    start_w: f32,
}

/// A horizontal split with draggable gutters.
pub struct Split {
    panes: Vec<Pane>,
    gutter: f32,
    gutter_rects: Vec<Rect>,
    hovered_gutter: Option<usize>,
    drag: Option<Drag>,
}

impl Split {
    pub fn horizontal() -> Self {
        Split {
            panes: Vec::new(),
            gutter: DEFAULT_GUTTER,
            gutter_rects: Vec::new(),
            hovered_gutter: None,
            drag: None,
        }
    }

    /// A fixed-width, resizable pane with default min/max.
    pub fn fixed(self, width: f32, widget: impl Widget + 'static) -> Self {
        self.fixed_range(width, DEFAULT_MIN, DEFAULT_MAX, widget)
    }

    /// A fixed-width, resizable pane with explicit min/max width.
    pub fn fixed_range(
        mut self,
        width: f32,
        min: f32,
        max: f32,
        widget: impl Widget + 'static,
    ) -> Self {
        self.panes.push(Pane {
            widget: Box::new(widget),
            mode: Mode::Fixed(width),
            min,
            max,
            rect: Rect::ZERO,
        });
        self
    }

    /// A flexible pane that absorbs the remaining width.
    pub fn flex(mut self, widget: impl Widget + 'static) -> Self {
        self.panes.push(Pane {
            widget: Box::new(widget),
            mode: Mode::Flex,
            min: 0.0,
            max: f32::INFINITY,
            rect: Rect::ZERO,
        });
        self
    }

    /// Set the gutter thickness in logical pixels.
    pub fn gutter(mut self, gutter: f32) -> Self {
        self.gutter = gutter;
        self
    }

    fn fixed_width(&self, i: usize) -> f32 {
        match self.panes[i].mode {
            Mode::Fixed(w) => w.clamp(self.panes[i].min, self.panes[i].max),
            Mode::Flex => 0.0,
        }
    }
}

impl Widget for Split {
    fn layout(&mut self, cx: &mut LayoutCx<'_>, constraints: Constraints) -> Size {
        let w = if constraints.max.width.is_finite() {
            constraints.max.width
        } else {
            900.0
        };
        let h = if constraints.max.height.is_finite() {
            constraints.max.height
        } else {
            600.0
        };

        let n = self.panes.len();
        let gutters = n.saturating_sub(1);
        let gutters_w = gutters as f32 * self.gutter;

        let mut sum_fixed = 0.0;
        let mut flex_count = 0;
        for i in 0..n {
            match self.panes[i].mode {
                Mode::Fixed(_) => sum_fixed += self.fixed_width(i),
                Mode::Flex => flex_count += 1,
            }
        }
        let flex_avail = (w - sum_fixed - gutters_w).max(0.0);
        let flex_w = if flex_count > 0 {
            flex_avail / flex_count as f32
        } else {
            0.0
        };

        self.gutter_rects.clear();
        let mut cursor = 0.0f32;
        for i in 0..n {
            let pw = match self.panes[i].mode {
                Mode::Fixed(_) => self.fixed_width(i),
                Mode::Flex => flex_w,
            };
            self.panes[i].rect = Rect::from_xywh(cursor, 0.0, pw, h);
            self.panes[i]
                .widget
                .layout(cx, Constraints::tight(Size::new(pw, h)));
            cursor += pw;
            if i < n - 1 {
                self.gutter_rects
                    .push(Rect::from_xywh(cursor, 0.0, self.gutter, h));
                cursor += self.gutter;
            }
        }

        constraints.constrain(Size::new(w, h))
    }

    fn paint(&mut self, cx: &mut PaintCx<'_>, bounds: Rect, scene: &mut Scene) {
        let p = &cx.theme.palette;
        for pane in &mut self.panes {
            pane.widget.paint(cx, absolute(bounds, pane.rect), scene);
        }
        for (i, gr) in self.gutter_rects.iter().enumerate() {
            let abs = absolute(bounds, *gr);
            let active = self.hovered_gutter == Some(i) || self.drag.is_some();
            let color = if active { p.accent } else { p.border };
            // A thin handle centered in the gutter.
            let handle = Rect::from_xywh(abs.center().x - 1.0, abs.top() + 6.0, 2.0, abs.height() - 12.0);
            scene.rounded_rect(handle, color, 1.0);
        }
    }

    fn event(&mut self, cx: &mut EventCx<'_>, bounds: Rect, event: &InputEvent) {
        // Gutter interaction (drag to resize).
        match event {
            InputEvent::PointerMoved { pos } => {
                self.hovered_gutter = self
                    .gutter_rects
                    .iter()
                    .position(|gr| absolute(bounds, *gr).expand(hit_pad()).contains(*pos));
                if let Some(d) = &self.drag {
                    let new_w =
                        (d.start_w + d.sign * (pos.x - d.start_x)).clamp(self.panes[d.pane].min, self.panes[d.pane].max);
                    self.panes[d.pane].mode = Mode::Fixed(new_w);
                }
            }
            InputEvent::PointerLeft => self.hovered_gutter = None,
            InputEvent::PointerPressed {
                pos,
                button: PointerButton::Primary,
            } => {
                for i in 0..self.gutter_rects.len() {
                    if absolute(bounds, self.gutter_rects[i]).expand(hit_pad()).contains(*pos) {
                        // The gutter resizes an adjacent fixed pane; the flex
                        // pane absorbs the change.
                        if matches!(self.panes[i].mode, Mode::Fixed(_)) {
                            self.drag = Some(Drag {
                                pane: i,
                                sign: 1.0,
                                start_x: pos.x,
                                start_w: self.fixed_width(i),
                            });
                        } else if matches!(self.panes[i + 1].mode, Mode::Fixed(_)) {
                            self.drag = Some(Drag {
                                pane: i + 1,
                                sign: -1.0,
                                start_x: pos.x,
                                start_w: self.fixed_width(i + 1),
                            });
                        }
                        return; // consumed by the gutter
                    }
                }
            }
            InputEvent::PointerReleased {
                button: PointerButton::Primary,
                ..
            } => {
                self.drag = None;
            }
            _ => {}
        }

        // Forward everything else to the pane children.
        for pane in &mut self.panes {
            pane.widget.event(cx, absolute(bounds, pane.rect), event);
        }
    }
}

/// Expand a gutter's hit-box so it is easy to grab.
fn hit_pad() -> baseui_core::Insets {
    baseui_core::Insets::symmetric(3.0, 0.0)
}

#[cfg(test)]
mod tests {
    use super::Split;
    use crate::event::{InputEvent, PointerButton};
    use crate::layout::Constraints;
    use crate::text::Fonts;
    use crate::theme::Theme;
    use crate::widget::{EventCx, LayoutCx, PaintCx, Widget};
    use baseui_core::paint::Scene;
    use baseui_core::{Point, Rect, Size};

    struct Null;
    impl Widget for Null {
        fn layout(&mut self, _cx: &mut LayoutCx<'_>, c: Constraints) -> Size {
            c.constrain(Size::ZERO)
        }
        fn paint(&mut self, _cx: &mut PaintCx<'_>, _b: Rect, _s: &mut Scene) {}
    }

    #[test]
    fn flex_absorbs_and_gutter_drag_resizes() {
        let Some(fonts) = Fonts::load() else {
            eprintln!("no system fonts; skipping");
            return;
        };
        let theme = Theme::dark();
        let mut split = Split::horizontal()
            .fixed(200.0, Null)
            .flex(Null)
            .fixed(200.0, Null);

        let mut lcx = LayoutCx {
            fonts: &fonts,
            theme: &theme,
        };
        let c = Constraints::tight(Size::new(1000.0, 500.0));
        split.layout(&mut lcx, c);

        // Two fixed panes (200 each) + two 6px gutters => flex = 588.
        assert_eq!(split.panes[0].rect.width(), 200.0);
        assert!((split.panes[1].rect.width() - 588.0).abs() < 0.5);

        // Drag gutter 0 (between fixed pane 0 and the flex pane) right by 50px.
        let bounds = Rect::from_xywh(0.0, 0.0, 1000.0, 500.0);
        let mut ecx = EventCx {
            fonts: &fonts,
            theme: &theme,
        };
        split.event(
            &mut ecx,
            bounds,
            &InputEvent::PointerPressed {
                pos: Point::new(203.0, 10.0),
                button: PointerButton::Primary,
            },
        );
        split.event(
            &mut ecx,
            bounds,
            &InputEvent::PointerMoved {
                pos: Point::new(253.0, 10.0),
            },
        );
        split.layout(&mut lcx, c);

        assert!((split.panes[0].rect.width() - 250.0).abs() < 0.5);
        // The flex pane shrinks by the same 50px.
        assert!((split.panes[1].rect.width() - 538.0).abs() < 0.5);
    }
}
