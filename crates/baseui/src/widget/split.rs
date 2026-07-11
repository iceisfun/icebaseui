//! [`Split`] — a row/column of panes separated by draggable gutters.
//!
//! Panes are either **fixed** (resizable by dragging their gutter) or **flexible**
//! (they absorb the remaining space, so content grows as the window grows). A
//! horizontal split lays panes left→right along their widths; a vertical split
//! lays them top→bottom along their heights. Nesting a horizontal split inside a
//! vertical one builds the classic app frame: menu / toolbar / [left | center |
//! right] / status.

use baseui_core::paint::Scene;
use baseui_core::{Insets, Point, Rect, Size};

use super::{EventCx, LayoutCx, PaintCx, Widget, absolute};
use crate::event::{InputEvent, PointerButton};
use crate::layout::Constraints;

const DEFAULT_GUTTER: f32 = 6.0;
const DEFAULT_MIN: f32 = 120.0;
const DEFAULT_MAX: f32 = 900.0;

/// The direction a [`Split`] arranges its panes.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Axis {
    Horizontal,
    Vertical,
}

impl Axis {
    /// The main-axis extent of a size.
    fn main(self, s: Size) -> f32 {
        match self {
            Axis::Horizontal => s.width,
            Axis::Vertical => s.height,
        }
    }

    /// The main-axis coordinate of a point.
    fn main_coord(self, p: Point) -> f32 {
        match self {
            Axis::Horizontal => p.x,
            Axis::Vertical => p.y,
        }
    }

    /// Build a pane rect from a main-axis offset+length and the full cross size.
    fn rect(self, main_pos: f32, main_len: f32, cross: f32) -> Rect {
        match self {
            Axis::Horizontal => Rect::from_xywh(main_pos, 0.0, main_len, cross),
            Axis::Vertical => Rect::from_xywh(0.0, main_pos, cross, main_len),
        }
    }
}

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
    /// Index of the fixed pane whose size the gutter controls.
    pane: usize,
    /// +1 if the controlled pane precedes the gutter, -1 if it follows.
    sign: f32,
    start_main: f32,
    start_len: f32,
}

/// A split container with draggable gutters. See the [module docs](self).
pub struct Split {
    axis: Axis,
    panes: Vec<Pane>,
    gutter: f32,
    gutter_rects: Vec<Rect>,
    hovered_gutter: Option<usize>,
    drag: Option<Drag>,
    persist_key: Option<String>,
}

impl Split {
    pub fn horizontal() -> Self {
        Self::new(Axis::Horizontal)
    }

    pub fn vertical() -> Self {
        Self::new(Axis::Vertical)
    }

    fn new(axis: Axis) -> Self {
        Split {
            axis,
            panes: Vec::new(),
            gutter: DEFAULT_GUTTER,
            gutter_rects: Vec::new(),
            hovered_gutter: None,
            drag: None,
            persist_key: None,
        }
    }

    /// Persist this split's pane sizes under `key` between runs.
    pub fn persist(mut self, key: impl Into<String>) -> Self {
        self.persist_key = Some(key.into());
        self
    }

    /// A fixed-size, resizable pane with default min/max.
    pub fn fixed(self, size: f32, widget: impl Widget + 'static) -> Self {
        self.fixed_range(size, DEFAULT_MIN, DEFAULT_MAX, widget)
    }

    /// A fixed-size, resizable pane with explicit min/max main-axis size.
    pub fn fixed_range(
        mut self,
        size: f32,
        min: f32,
        max: f32,
        widget: impl Widget + 'static,
    ) -> Self {
        self.panes.push(Pane {
            widget: Box::new(widget),
            mode: Mode::Fixed(size),
            min,
            max,
            rect: Rect::ZERO,
        });
        self
    }

    /// A flexible pane that absorbs the remaining main-axis space.
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

    fn fixed_len(&self, i: usize) -> f32 {
        match self.panes[i].mode {
            Mode::Fixed(v) => v.clamp(self.panes[i].min, self.panes[i].max),
            Mode::Flex => 0.0,
        }
    }

    fn hit_pad(&self) -> Insets {
        match self.axis {
            Axis::Horizontal => Insets::symmetric(3.0, 0.0),
            Axis::Vertical => Insets::symmetric(0.0, 3.0),
        }
    }
}

impl Widget for Split {
    fn layout(&mut self, cx: &mut LayoutCx<'_>, constraints: Constraints) -> Size {
        let bounded = |v: f32, fallback: f32| if v.is_finite() { v } else { fallback };
        let w = bounded(constraints.max.width, 900.0);
        let h = bounded(constraints.max.height, 600.0);
        let full = Size::new(w, h);
        let main_total = self.axis.main(full);
        let cross = match self.axis {
            Axis::Horizontal => h,
            Axis::Vertical => w,
        };

        let n = self.panes.len();
        let gutters = n.saturating_sub(1);
        let gutters_len = gutters as f32 * self.gutter;

        let mut sum_fixed = 0.0;
        let mut flex_count = 0;
        for i in 0..n {
            match self.panes[i].mode {
                Mode::Fixed(_) => sum_fixed += self.fixed_len(i),
                Mode::Flex => flex_count += 1,
            }
        }
        let flex_avail = (main_total - sum_fixed - gutters_len).max(0.0);
        let flex_len = if flex_count > 0 {
            flex_avail / flex_count as f32
        } else {
            0.0
        };

        self.gutter_rects.clear();
        let mut cursor = 0.0f32;
        for i in 0..n {
            let len = match self.panes[i].mode {
                Mode::Fixed(_) => self.fixed_len(i),
                Mode::Flex => flex_len,
            };
            let rect = self.axis.rect(cursor, len, cross);
            self.panes[i].rect = rect;
            self.panes[i]
                .widget
                .layout(cx, Constraints::tight(rect.size));
            cursor += len;
            if i < n - 1 {
                self.gutter_rects
                    .push(self.axis.rect(cursor, self.gutter, cross));
                cursor += self.gutter;
            }
        }

        constraints.constrain(full)
    }

    fn paint(&mut self, cx: &mut PaintCx<'_>, bounds: Rect, scene: &mut Scene) {
        let p = &cx.theme.palette;
        for pane in &mut self.panes {
            pane.widget.paint(cx, absolute(bounds, pane.rect), scene);
        }
        if self.gutter <= 0.0 {
            return; // inert split (e.g. a fixed bar stack): no visible gutters
        }
        for (i, gr) in self.gutter_rects.iter().enumerate() {
            let abs = absolute(bounds, *gr);
            let active = self.hovered_gutter == Some(i) || self.drag.is_some();
            let color = if active { p.accent } else { p.border };
            // A slim handle centered along the gutter.
            let handle = match self.axis {
                Axis::Horizontal => Rect::from_xywh(
                    abs.center().x - 1.0,
                    abs.top() + 6.0,
                    2.0,
                    (abs.height() - 12.0).max(0.0),
                ),
                Axis::Vertical => Rect::from_xywh(
                    abs.left() + 6.0,
                    abs.center().y - 1.0,
                    (abs.width() - 12.0).max(0.0),
                    2.0,
                ),
            };
            scene.rounded_rect(handle, color, 1.0);
        }
    }

    fn event(&mut self, cx: &mut EventCx<'_>, bounds: Rect, event: &InputEvent) {
        let pad = self.hit_pad();
        let draggable = self.gutter > 0.0;
        match event {
            InputEvent::PointerMoved { pos } if draggable => {
                self.hovered_gutter = self
                    .gutter_rects
                    .iter()
                    .position(|gr| absolute(bounds, *gr).expand(pad).contains(*pos));
                if let Some(d) = &self.drag {
                    let delta = self.axis.main_coord(*pos) - d.start_main;
                    let new_len = (d.start_len + d.sign * delta)
                        .clamp(self.panes[d.pane].min, self.panes[d.pane].max);
                    self.panes[d.pane].mode = Mode::Fixed(new_len);
                }
            }
            InputEvent::PointerLeft => self.hovered_gutter = None,
            InputEvent::PointerPressed {
                pos,
                button: PointerButton::Primary,
            } if draggable => {
                for i in 0..self.gutter_rects.len() {
                    if absolute(bounds, self.gutter_rects[i]).expand(pad).contains(*pos) {
                        // Resize an adjacent fixed pane; the flex pane absorbs it.
                        if matches!(self.panes[i].mode, Mode::Fixed(_)) {
                            self.drag = Some(Drag {
                                pane: i,
                                sign: 1.0,
                                start_main: self.axis.main_coord(*pos),
                                start_len: self.fixed_len(i),
                            });
                        } else if matches!(self.panes[i + 1].mode, Mode::Fixed(_)) {
                            self.drag = Some(Drag {
                                pane: i + 1,
                                sign: -1.0,
                                start_main: self.axis.main_coord(*pos),
                                start_len: self.fixed_len(i + 1),
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

        for pane in &mut self.panes {
            let ev = cx.effective(event);
            pane.widget.event(cx, absolute(bounds, pane.rect), ev);
        }
    }

    fn persist_save(&self, store: &mut crate::persist::Store) {
        if let Some(key) = &self.persist_key {
            let sizes: Vec<f32> = self
                .panes
                .iter()
                .map(|p| match p.mode {
                    Mode::Fixed(v) => v,
                    Mode::Flex => 0.0,
                })
                .collect();
            store.set(key.clone(), &sizes);
        }
        for pane in &self.panes {
            pane.widget.persist_save(store);
        }
    }

    fn persist_restore(&mut self, store: &crate::persist::Store) {
        if let Some(key) = &self.persist_key {
            if let Some(sizes) = store.get::<Vec<f32>>(key) {
                for (i, pane) in self.panes.iter_mut().enumerate() {
                    if let Mode::Fixed(_) = pane.mode {
                        if let Some(v) = sizes.get(i) {
                            pane.mode = Mode::Fixed(v.clamp(pane.min, pane.max));
                        }
                    }
                }
            }
        }
        for pane in &mut self.panes {
            pane.widget.persist_restore(store);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Mode, Split};
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
            window: None,
        };
        let c = Constraints::tight(Size::new(1000.0, 500.0));
        split.layout(&mut lcx, c);

        assert_eq!(split.panes[0].rect.width(), 200.0);
        assert!((split.panes[1].rect.width() - 588.0).abs() < 0.5);

        let bounds = Rect::from_xywh(0.0, 0.0, 1000.0, 500.0);
        let mut ecx = EventCx::new(&fonts, &theme, Size::new(1000.0, 1000.0));
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
        assert!((split.panes[1].rect.width() - 538.0).abs() < 0.5);
    }

    #[test]
    fn vertical_stacks_and_flex_absorbs_height() {
        let Some(fonts) = Fonts::load() else {
            return;
        };
        let theme = Theme::dark();
        let mut split = Split::vertical()
            .fixed_range(40.0, 40.0, 40.0, Null)
            .flex(Null)
            .fixed_range(24.0, 24.0, 24.0, Null);
        let mut lcx = LayoutCx {
            fonts: &fonts,
            theme: &theme,
            window: None,
        };
        split.layout(&mut lcx, Constraints::tight(Size::new(800.0, 600.0)));
        assert_eq!(split.panes[0].rect.height(), 40.0);
        assert_eq!(split.panes[2].rect.height(), 24.0);
        // 600 - 40 - 24 - 2*6 gutters = 524.
        assert!((split.panes[1].rect.height() - 524.0).abs() < 0.5);
        // Cross axis fills the full width.
        assert_eq!(split.panes[1].rect.width(), 800.0);
    }

    #[test]
    fn persist_roundtrips_pane_sizes() {
        let mut store = crate::persist::Store::new();
        let saved = Split::horizontal()
            .persist("split.test")
            .fixed(300.0, Null)
            .flex(Null)
            .fixed(200.0, Null);
        saved.persist_save(&mut store);

        // A fresh split with different sizes restores to the saved ones.
        let mut restored = Split::horizontal()
            .persist("split.test")
            .fixed(120.0, Null)
            .flex(Null)
            .fixed(120.0, Null);
        restored.persist_restore(&store);
        assert!(matches!(restored.panes[0].mode, Mode::Fixed(v) if (v - 300.0).abs() < 0.01));
        assert!(matches!(restored.panes[2].mode, Mode::Fixed(v) if (v - 200.0).abs() < 0.01));
    }
}
