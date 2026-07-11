//! [`TreeView`] and [`TreeNode`] — a collapsible, selectable hierarchy.
//!
//! Models Blender's Outliner (see `docs`/the project's reference): nested nodes
//! with expand/collapse arrows, colored type icons, a full-row selection
//! highlight, hover feedback, and unlimited depth. Place it inside a
//! [`ScrollArea`](super::ScrollArea) for long trees.
//!
//! Deferred to later milestones: keyboard navigation, drag-and-drop, rename,
//! multi-select, filtering, badges, and context menus (all in the SOW).

use baseui_core::paint::Scene;
use baseui_core::{Color, Id, Point, Rect, Size};

use super::{EventCx, LayoutCx, PaintCx, Widget};
use crate::event::{InputEvent, PointerButton};
use crate::layout::Constraints;
use crate::text::FontId;

const INDENT: f32 = 14.0;
const ARROW_W: f32 = 14.0;
const ICON_W: f32 = 14.0;

/// One node in a [`TreeView`]. Build leaves with [`TreeNode::leaf`] and parents
/// with [`TreeNode::branch`].
pub struct TreeNode {
    id: Id,
    label: String,
    icon_color: Option<Color>,
    children: Vec<TreeNode>,
    expanded: bool,
}

impl TreeNode {
    /// A childless node.
    pub fn leaf(label: impl Into<String>) -> Self {
        TreeNode {
            id: Id::next(),
            label: label.into(),
            icon_color: None,
            children: Vec::new(),
            expanded: false,
        }
    }

    /// A node with children (expanded by default).
    pub fn branch(label: impl Into<String>, children: Vec<TreeNode>) -> Self {
        TreeNode {
            id: Id::next(),
            label: label.into(),
            icon_color: None,
            children,
            expanded: true,
        }
    }

    /// Set the type-icon color (a colored dot beside the label).
    pub fn icon_color(mut self, color: Color) -> Self {
        self.icon_color = Some(color);
        self
    }

    /// Start collapsed.
    pub fn collapsed(mut self) -> Self {
        self.expanded = false;
        self
    }

    fn has_children(&self) -> bool {
        !self.children.is_empty()
    }

    fn find_mut(&mut self, id: Id) -> Option<&mut TreeNode> {
        if self.id == id {
            return Some(self);
        }
        for child in &mut self.children {
            if let Some(found) = child.find_mut(id) {
                return Some(found);
            }
        }
        None
    }
}

/// A flattened, currently-visible row (recomputed each layout).
struct FlatRow {
    id: Id,
    depth: usize,
    label: String,
    icon_color: Option<Color>,
    has_children: bool,
    expanded: bool,
}

fn flatten(nodes: &[TreeNode], depth: usize, out: &mut Vec<FlatRow>) {
    for node in nodes {
        out.push(FlatRow {
            id: node.id,
            depth,
            label: node.label.clone(),
            icon_color: node.icon_color,
            has_children: node.has_children(),
            expanded: node.expanded,
        });
        if node.expanded {
            flatten(&node.children, depth + 1, out);
        }
    }
}

/// Callback invoked with a node's label when it is selected.
type SelectFn = Box<dyn FnMut(&str)>;

/// A hierarchical list with expand/collapse and single selection.
pub struct TreeView {
    roots: Vec<TreeNode>,
    selected: Option<Id>,
    hovered: Option<Id>,
    on_select: Option<SelectFn>,
    font_size: f32,
    row_h: f32,
    rows: Vec<FlatRow>,
}

impl TreeView {
    pub fn new(roots: Vec<TreeNode>) -> Self {
        TreeView {
            roots,
            selected: None,
            hovered: None,
            on_select: None,
            font_size: 13.0,
            row_h: 22.0,
            rows: Vec::new(),
        }
    }

    /// Called with the label of a node when it becomes selected.
    pub fn on_select(mut self, f: impl FnMut(&str) + 'static) -> Self {
        self.on_select = Some(Box::new(f));
        self
    }

    fn toggle(&mut self, id: Id) {
        for root in &mut self.roots {
            if let Some(node) = root.find_mut(id) {
                node.expanded = !node.expanded;
                return;
            }
        }
    }

    /// Row index under a y coordinate relative to the widget top.
    fn row_at(&self, y_rel: f32) -> Option<usize> {
        if y_rel < 0.0 {
            return None;
        }
        let i = (y_rel / self.row_h) as usize;
        (i < self.rows.len()).then_some(i)
    }
}

impl Widget for TreeView {
    fn layout(&mut self, cx: &mut LayoutCx<'_>, constraints: Constraints) -> Size {
        self.row_h = cx.fonts.line_height(self.font_size, FontId::Ui) + 8.0;
        self.rows.clear();
        flatten(&self.roots, 0, &mut self.rows);

        let width = if constraints.max.width.is_finite() {
            constraints.max.width
        } else {
            240.0
        };
        let height = self.rows.len() as f32 * self.row_h;
        constraints.constrain(Size::new(width, height))
    }

    fn paint(&mut self, cx: &mut PaintCx<'_>, bounds: Rect, scene: &mut Scene) {
        let p = &cx.theme.palette;
        let radius = cx.theme.radius.sm;

        for (i, row) in self.rows.iter().enumerate() {
            let y = bounds.top() + i as f32 * self.row_h;
            let row_rect = Rect::from_xywh(bounds.left() + 2.0, y, bounds.width() - 4.0, self.row_h);

            // Selection / hover backgrounds.
            if self.selected == Some(row.id) {
                scene.rounded_rect(row_rect, p.selection, radius);
            } else if self.hovered == Some(row.id) {
                scene.rounded_rect(row_rect, p.hover, radius);
            }

            let base_x = bounds.left() + cx.theme.spacing.sm + row.depth as f32 * INDENT;
            let text_y = y + (self.row_h - cx.fonts.line_height(self.font_size, FontId::Ui)) * 0.5;

            // Expand/collapse arrow.
            if row.has_children {
                let arrow = if row.expanded { "\u{25BE}" } else { "\u{25B8}" };
                scene.text(Point::new(base_x, text_y), arrow, self.font_size, p.text_muted);
            }

            // Colored type icon (a rounded square).
            let icon_x = base_x + ARROW_W;
            let dot = 9.0;
            scene.rounded_rect(
                Rect::from_xywh(icon_x, y + (self.row_h - dot) * 0.5, dot, dot),
                row.icon_color.unwrap_or(p.text_muted),
                2.5,
            );

            // Label.
            let label_x = icon_x + ICON_W + cx.theme.spacing.xs;
            scene.text(
                Point::new(label_x, text_y),
                row.label.clone(),
                self.font_size,
                p.text,
            );
        }
    }

    fn event(&mut self, cx: &mut EventCx<'_>, bounds: Rect, event: &InputEvent) {
        match event {
            InputEvent::PointerMoved { pos } => {
                self.hovered = self
                    .row_at(pos.y - bounds.top())
                    .filter(|_| bounds.contains(*pos))
                    .map(|i| self.rows[i].id);
            }
            InputEvent::PointerLeft => self.hovered = None,
            InputEvent::PointerPressed {
                pos,
                button: PointerButton::Primary,
            } => {
                if !bounds.contains(*pos) {
                    return;
                }
                let Some(i) = self.row_at(pos.y - bounds.top()) else {
                    return;
                };
                let row_depth = self.rows[i].depth;
                let has_children = self.rows[i].has_children;
                let id = self.rows[i].id;

                // Arrow hit-box toggles expansion; anywhere else selects.
                let arrow_x = bounds.left() + cx.theme.spacing.sm + row_depth as f32 * INDENT;
                if has_children && pos.x >= arrow_x && pos.x < arrow_x + ARROW_W {
                    self.toggle(id);
                } else {
                    self.selected = Some(id);
                    if let Some(cb) = self.on_select.as_mut() {
                        let label = self.rows[i].label.clone();
                        cb(&label);
                    }
                }
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flatten_respects_expansion() {
        let mut nodes = vec![TreeNode::branch(
            "root",
            vec![TreeNode::leaf("a"), TreeNode::leaf("b")],
        )];
        let mut rows = Vec::new();
        flatten(&nodes, 0, &mut rows);
        assert_eq!(rows.len(), 3);
        assert_eq!(rows[0].depth, 0);
        assert_eq!(rows[1].depth, 1);
        assert!(rows[0].has_children);

        // Collapsing the root hides its children.
        nodes[0].expanded = false;
        rows.clear();
        flatten(&nodes, 0, &mut rows);
        assert_eq!(rows.len(), 1);
    }

    #[test]
    fn toggle_flips_expansion() {
        let mut tv = TreeView::new(vec![TreeNode::branch("root", vec![TreeNode::leaf("a")])]);
        let id = tv.roots[0].id;
        assert!(tv.roots[0].expanded);
        tv.toggle(id);
        assert!(!tv.roots[0].expanded);
        tv.toggle(id);
        assert!(tv.roots[0].expanded);
    }
}
