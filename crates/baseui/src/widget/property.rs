//! [`PropertyView`], [`PropGroup`], and property rows — a grouped inspector.
//!
//! Models Blender's Properties editor: collapsible groups with colored section
//! icons, each containing label/editor rows. The editor for a row is any
//! [`Widget`] — typically a [`DragValue`](super::DragValue),
//! [`Slider`](super::Slider), or [`Checkbox`](super::Checkbox) bound to a
//! signal — so the property system reuses the core widgets rather than
//! reimplementing editors. Place it inside a [`ScrollArea`](super::ScrollArea).
//!
//! Deferred: search/filtering, inline validation, reset buttons, read-only
//! styling, and arbitrarily deep nesting (all in the SOW).

use baseui_core::paint::{RectShape, Scene};
use baseui_core::{Color, Id, Point, Rect, Size};

use super::{EventCx, LayoutCx, PaintCx, Widget, absolute};
use crate::event::{InputEvent, PointerButton};
use crate::layout::Constraints;
use crate::text::FontId;

/// One label + editor row inside a [`PropGroup`].
pub struct PropRow {
    label: String,
    editor: Box<dyn Widget>,
    label_pos: Point,
    editor_rect: Rect,
}

/// A collapsible section of property rows.
pub struct PropGroup {
    id: Id,
    title: String,
    icon_color: Option<Color>,
    collapsed: bool,
    rows: Vec<PropRow>,
    header_rect: Rect,
}

impl PropGroup {
    pub fn new(title: impl Into<String>) -> Self {
        PropGroup {
            id: Id::next(),
            title: title.into(),
            icon_color: None,
            collapsed: false,
            rows: Vec::new(),
            header_rect: Rect::ZERO,
        }
    }

    /// Colored section icon.
    pub fn icon_color(mut self, color: Color) -> Self {
        self.icon_color = Some(color);
        self
    }

    /// Start collapsed.
    pub fn collapsed(mut self) -> Self {
        self.collapsed = true;
        self
    }

    /// Add a labelled editor row.
    pub fn row(mut self, label: impl Into<String>, editor: impl Widget + 'static) -> Self {
        self.rows.push(PropRow {
            label: label.into(),
            editor: Box::new(editor),
            label_pos: Point::ZERO,
            editor_rect: Rect::ZERO,
        });
        self
    }
}

/// A grouped property inspector.
pub struct PropertyView {
    groups: Vec<PropGroup>,
    font_size: f32,
    /// Fraction of the width used by the label column.
    label_fraction: f32,
    hovered_header: Option<Id>,
    persist_key: Option<String>,
}

impl PropertyView {
    pub fn new() -> Self {
        PropertyView {
            groups: Vec::new(),
            font_size: 13.0,
            label_fraction: 0.42,
            hovered_header: None,
            persist_key: None,
        }
    }

    pub fn group(mut self, group: PropGroup) -> Self {
        self.groups.push(group);
        self
    }

    /// Persist which groups are collapsed under `key` between runs.
    pub fn persist(mut self, key: impl Into<String>) -> Self {
        self.persist_key = Some(key.into());
        self
    }
}

impl Default for PropertyView {
    fn default() -> Self {
        PropertyView::new()
    }
}

impl Widget for PropertyView {
    fn layout(&mut self, cx: &mut LayoutCx<'_>, constraints: Constraints) -> Size {
        let w = if constraints.max.width.is_finite() {
            constraints.max.width
        } else {
            320.0
        };
        let line_h = cx.fonts.line_height(self.font_size, FontId::Ui);
        let header_h = line_h + cx.theme.spacing.md;
        let min_row_h = line_h + cx.theme.spacing.sm * 2.0;
        let pad = cx.theme.spacing.md;
        let label_col_w = w * self.label_fraction;
        let editor_x = label_col_w + pad;
        let editor_avail = (w - editor_x - pad).max(20.0);

        let mut y = 0.0f32;
        for group in &mut self.groups {
            group.header_rect = Rect::from_xywh(0.0, y, w, header_h);
            y += header_h;

            if !group.collapsed {
                for row in &mut group.rows {
                    let es = row
                        .editor
                        .layout(cx, Constraints::loose(Size::new(editor_avail, f32::INFINITY)));
                    let row_h = min_row_h.max(es.height + cx.theme.spacing.xs * 2.0);
                    row.label_pos = Point::new(pad, y + (row_h - line_h) * 0.5);
                    row.editor_rect = Rect::from_xywh(
                        editor_x,
                        y + (row_h - es.height) * 0.5,
                        es.width.min(editor_avail),
                        es.height,
                    );
                    y += row_h;
                }
                y += cx.theme.spacing.xs;
            }
        }

        constraints.constrain(Size::new(w, y))
    }

    fn paint(&mut self, cx: &mut PaintCx<'_>, bounds: Rect, scene: &mut Scene) {
        let p = &cx.theme.palette;
        let line_h = cx.fonts.line_height(self.font_size, FontId::Ui);

        for group in &mut self.groups {
            let hr = absolute(bounds, group.header_rect);

            let bg = if self.hovered_header == Some(group.id) {
                p.hover
            } else {
                p.surface_variant
            };
            scene.push_rect(RectShape::fill(hr, bg).with_corner_radius(cx.theme.radius.sm));

            let cy = hr.top() + (hr.height() - line_h) * 0.5;
            let arrow = if group.collapsed { "\u{25B8}" } else { "\u{25BE}" };
            scene.text(
                Point::new(hr.left() + cx.theme.spacing.md, cy),
                arrow,
                self.font_size,
                p.text_muted,
            );
            let icon_x = hr.left() + cx.theme.spacing.md + 14.0;
            let dot = 10.0;
            scene.rounded_rect(
                Rect::from_xywh(icon_x, hr.top() + (hr.height() - dot) * 0.5, dot, dot),
                group.icon_color.unwrap_or(p.accent),
                2.5,
            );
            scene.text(
                Point::new(icon_x + 16.0, cy),
                group.title.clone(),
                self.font_size,
                p.text,
            );

            if !group.collapsed {
                for row in &mut group.rows {
                    scene.text(
                        Point::new(
                            bounds.left() + row.label_pos.x,
                            bounds.top() + row.label_pos.y,
                        ),
                        row.label.clone(),
                        self.font_size,
                        p.text_muted,
                    );
                    row.editor.paint(cx, absolute(bounds, row.editor_rect), scene);
                }
            }
        }
    }

    fn event(&mut self, cx: &mut EventCx<'_>, bounds: Rect, event: &InputEvent) {
        // Header hover + collapse toggling.
        match event {
            InputEvent::PointerMoved { pos } => {
                self.hovered_header = self
                    .groups
                    .iter()
                    .find(|g| absolute(bounds, g.header_rect).contains(*pos))
                    .map(|g| g.id);
            }
            InputEvent::PointerLeft => self.hovered_header = None,
            InputEvent::PointerPressed {
                pos,
                button: PointerButton::Primary,
            } => {
                for group in &mut self.groups {
                    if absolute(bounds, group.header_rect).contains(*pos) {
                        group.collapsed = !group.collapsed;
                        return;
                    }
                }
            }
            _ => {}
        }

        // Route events to editors of expanded groups.
        for group in &mut self.groups {
            if group.collapsed {
                continue;
            }
            for row in &mut group.rows {
                row.editor.event(cx, absolute(bounds, row.editor_rect), event);
            }
        }
    }

    fn persist_save(&self, store: &mut crate::persist::Store) {
        if let Some(key) = &self.persist_key {
            let collapsed: Vec<bool> = self.groups.iter().map(|g| g.collapsed).collect();
            store.set(key.clone(), &collapsed);
        }
    }

    fn persist_restore(&mut self, store: &crate::persist::Store) {
        if let Some(key) = &self.persist_key {
            if let Some(collapsed) = store.get::<Vec<bool>>(key) {
                for (group, &c) in self.groups.iter_mut().zip(&collapsed) {
                    group.collapsed = c;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::text::Fonts;
    use crate::theme::Theme;
    use crate::widget::DragValue;
    use baseui_core::create_signal;

    #[test]
    fn collapsing_a_group_shrinks_height() {
        let Some(fonts) = Fonts::load() else {
            eprintln!("no system fonts; skipping");
            return;
        };
        let theme = Theme::dark();
        let sig = create_signal(0.0f32);
        let mut pv = PropertyView::new().group(
            PropGroup::new("Transform")
                .row("X", DragValue::new(sig))
                .row("Y", DragValue::new(sig))
                .row("Z", DragValue::new(sig)),
        );

        let mut lcx = LayoutCx {
            fonts: &fonts,
            theme: &theme,
        };
        let c = Constraints::loose(Size::new(320.0, f32::INFINITY));
        let expanded = pv.layout(&mut lcx, c);

        pv.groups[0].collapsed = true;
        let collapsed = pv.layout(&mut lcx, c);

        assert!(collapsed.height < expanded.height);
        assert_eq!(collapsed.width, expanded.width);
    }
}
