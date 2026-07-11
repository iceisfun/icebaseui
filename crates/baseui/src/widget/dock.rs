//! [`DockArea`] — the docking workspace: tab groups, recursive splits,
//! drag-reorder, drag-to-split, and persistent layouts.
//!
//! # The one design decision
//!
//! **Layout and content are separate.**
//!
//! - [`DockNode`] is the *layout* tree, and it holds only **panel ids**. It is
//!   cheap to move, reorder, split, and serialize.
//! - [`Panel`]s (the actual widgets) live in a flat registry keyed by id.
//!
//! So moving a tab — reorder, split, or (Phase 3) detach into another window — is
//! just **editing an id tree**. No re-parenting a `Box<dyn Widget>` out of one
//! owner into another, which is where docking implementations usually turn into
//! an ownership fight. Persistence falls out for free: the id tree is plain data.
//!
//! # Layout pipeline
//!
//! The recursive tree is *flattened* each layout into a list of [`GroupLayout`]s
//! (a tab strip + a content rect) and gutters. Paint and event then work on flat
//! lists; mutations are applied back to the tree by **path** (`Vec<usize>`).

use std::cell::RefCell;
use std::collections::HashMap;

use baseui_core::paint::{RectShape, Scene};
use baseui_core::{Insets, Point, Rect, Size};
use serde::{Deserialize, Serialize};

use super::{EventCx, LayoutCx, MenuItemSpec, PaintCx, PopupMenu, Widget};
use crate::event::{InputEvent, PointerButton};
use crate::icon::{Icon, glyphs};
use crate::layout::Constraints;
use crate::text::FontId;
use crate::window::{self, WindowSpec};

thread_local! {
    /// Panels handed back by a floating window, waiting to be re-absorbed by the
    /// DockArea on its next layout. Content moves as an owned `Panel`, so the
    /// widget itself is never duplicated or re-parented behind anyone's back.
    static REDOCK: RefCell<Vec<Panel>> = const { RefCell::new(Vec::new()) };
}

fn queue_redock(panel: Panel) {
    REDOCK.with(|r| r.borrow_mut().push(panel));
    window::mark_dirty();
}

fn take_redock() -> Vec<Panel> {
    REDOCK.with(|r| std::mem::take(&mut *r.borrow_mut()))
}

thread_local! {
    /// Windows asked (by command) to dock their panel back. The FloatingPanel
    /// consumes its own request at layout time.
    static REDOCK_REQUESTS: RefCell<Vec<window::WindowId>> = const { RefCell::new(Vec::new()) };
}

fn take_redock_request(id: Option<window::WindowId>) -> bool {
    let Some(id) = id else { return false };
    REDOCK_REQUESTS.with(|r| {
        let mut r = r.borrow_mut();
        if let Some(i) = r.iter().position(|w| *w == id) {
            r.remove(i);
            true
        } else {
            false
        }
    })
}

/// Register the commands a **detached panel window** offers. They carry the
/// `"panel"` context, so they appear in *that* window's Command Palette (and its
/// shortcuts fire there) but not in the main window's — one registry, scoped
/// visibility.
fn register_panel_commands() {
    crate::command::register(
        crate::command::CommandMeta::new("panel.dock", "Dock Panel Back")
            .category("Panel")
            .context("panel")
            .icon(glyphs::SQUARE)
            .shortcut("Ctrl+D"),
        || {
            if let Some(id) = window::focused() {
                REDOCK_REQUESTS.with(|r| r.borrow_mut().push(id));
                window::mark_dirty();
            }
        },
    );
    crate::command::register(
        crate::command::CommandMeta::new("panel.close_window", "Close This Panel Window")
            .category("Panel")
            .context("panel")
            .icon(glyphs::CROSS),
        || {
            if let Some(id) = window::focused() {
                window::close(id);
            }
        },
    );
}

/// Which way a dock split divides its children.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum DockAxis {
    Horizontal,
    Vertical,
}

/// The dock **layout** tree. Holds panel ids only — never widgets.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum DockNode {
    /// Children laid out along `axis`, sized by `sizes` (fractions summing to 1).
    Split {
        axis: DockAxis,
        children: Vec<DockNode>,
        sizes: Vec<f32>,
    },
    /// A tab group: several panels sharing one area, one of them active.
    Tabs { panels: Vec<String>, active: usize },
}

impl DockNode {
    /// A tab group holding these panel ids.
    pub fn tabs(panels: impl IntoIterator<Item = impl Into<String>>) -> Self {
        DockNode::Tabs {
            panels: panels.into_iter().map(Into::into).collect(),
            active: 0,
        }
    }

    /// A split with equally-sized children.
    pub fn split(axis: DockAxis, children: Vec<DockNode>) -> Self {
        let n = children.len().max(1);
        let sizes = vec![1.0 / n as f32; children.len()];
        DockNode::Split {
            axis,
            children,
            sizes,
        }
    }

    fn collect_panels(&self, out: &mut Vec<String>) {
        match self {
            DockNode::Tabs { panels, .. } => out.extend(panels.iter().cloned()),
            DockNode::Split { children, .. } => {
                for c in children {
                    c.collect_panels(out);
                }
            }
        }
    }
}

/// A dockable panel: an id, its tab chrome, and the widget it shows.
pub struct Panel {
    id: String,
    title: String,
    icon: Option<Icon>,
    closable: bool,
    widget: Box<dyn Widget>,
}

impl Panel {
    pub fn new(id: impl Into<String>, title: impl Into<String>, widget: impl Widget + 'static) -> Self {
        Panel {
            id: id.into(),
            title: title.into(),
            icon: None,
            closable: true,
            widget: Box::new(widget),
        }
    }

    pub fn icon(mut self, icon: Icon) -> Self {
        self.icon = Some(icon);
        self
    }

    /// A panel that cannot be closed (no × on its tab).
    pub fn pinned(mut self) -> Self {
        self.closable = false;
        self
    }
}

/// Where a dragged tab would land.
#[derive(Clone, PartialEq, Debug)]
enum DropZone {
    /// Into a tab strip, at this insert index.
    Tab(usize),
    /// Split the target group, putting the panel on this side.
    Split(Side),
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Side {
    Left,
    Right,
    Top,
    Bottom,
}

#[derive(Clone, Debug)]
struct DropTarget {
    path: Vec<usize>,
    zone: DropZone,
}

/// An in-progress tab drag.
struct TabDrag {
    from: Vec<usize>,
    tab: usize,
    panel: String,
    start: Point,
    pos: Point,
    /// Only a real drag once the pointer has moved past a threshold.
    active: bool,
    target: Option<DropTarget>,
}

/// A flattened tab group produced by the layout walk.
struct GroupLayout {
    path: Vec<usize>,
    rect: Rect,
    strip: Rect,
    content: Rect,
    tabs: Vec<Rect>,
    panels: Vec<String>,
    active: usize,
}

/// A flattened split gutter.
struct GutterLayout {
    path: Vec<usize>,
    /// Index of the child *before* the gutter.
    index: usize,
    rect: Rect,
    axis: DockAxis,
}

struct GutterDrag {
    path: Vec<usize>,
    index: usize,
    axis: DockAxis,
    start: Point,
    /// Fractions of the two children the gutter sits between, at drag start.
    start_sizes: (f32, f32),
    /// Total main-axis length of the split, for converting pixels to fractions.
    total: f32,
}

const GUTTER: f32 = 6.0;
const DRAG_THRESHOLD: f32 = 4.0;
/// Fraction of a group's edge that triggers a split rather than a tab drop.
const EDGE: f32 = 0.25;

/// The docking workspace.
pub struct DockArea {
    root: DockNode,
    panels: HashMap<String, Panel>,
    font_size: f32,

    groups: Vec<GroupLayout>,
    gutters: Vec<GutterLayout>,

    hovered_tab: Option<(usize, usize)>,
    hovered_close: Option<(usize, usize)>,
    hovered_gutter: Option<usize>,
    drag: Option<TabDrag>,
    gutter_drag: Option<GutterDrag>,

    context: PopupMenu,
    context_panel: Option<String>,

    persist_key: Option<String>,
}

impl DockArea {
    pub fn new(root: DockNode) -> Self {
        register_panel_commands();
        DockArea {
            root,
            panels: HashMap::new(),
            font_size: 13.0,
            groups: Vec::new(),
            gutters: Vec::new(),
            hovered_tab: None,
            hovered_close: None,
            hovered_gutter: None,
            drag: None,
            gutter_drag: None,
            context: PopupMenu::new(),
            context_panel: None,
            persist_key: None,
        }
    }

    /// Register a panel's content. Panels not present in the layout tree are
    /// appended to the first tab group, so registering one is enough to show it.
    pub fn panel(mut self, panel: Panel) -> Self {
        self.panels.insert(panel.id.clone(), panel);
        self
    }

    /// Persist the layout tree (splits, sizes, tab order, active tabs) under `key`.
    pub fn persist(mut self, key: impl Into<String>) -> Self {
        self.persist_key = Some(key.into());
        self
    }

    fn strip_height(&self, cx_fonts: &crate::text::Fonts) -> f32 {
        cx_fonts.line_height(self.font_size, FontId::Ui) + 10.0 * crate::text::scale()
    }

    /// Width of one tab, from its panel's chrome.
    fn tab_width(&self, fonts: &crate::text::Fonts, id: &str) -> f32 {
        let s = crate::text::scale();
        let Some(panel) = self.panels.get(id) else {
            return 60.0 * s;
        };
        let mut w = fonts.measure(&panel.title, self.font_size, FontId::Ui).width + 20.0 * s;
        if let Some(icon) = panel.icon {
            w += fonts.char_advance(icon.ch(), self.font_size, icon.font_id()) + 6.0 * s;
        }
        if panel.closable {
            w += 18.0 * s;
        }
        w
    }

    /// Flatten the tree into groups + gutters for `rect`.
    #[allow(clippy::too_many_arguments)]
    fn walk(
        &self,
        node: &DockNode,
        rect: Rect,
        path: &mut Vec<usize>,
        fonts: &crate::text::Fonts,
        strip_h: f32,
        groups: &mut Vec<GroupLayout>,
        gutters: &mut Vec<GutterLayout>,
    ) {
        match node {
            DockNode::Tabs { panels, active } => {
                let strip = Rect::from_xywh(rect.left(), rect.top(), rect.width(), strip_h);
                let content = Rect::from_xywh(
                    rect.left(),
                    rect.top() + strip_h,
                    rect.width(),
                    (rect.height() - strip_h).max(0.0),
                );
                let mut tabs = Vec::with_capacity(panels.len());
                let mut x = strip.left();
                for id in panels {
                    let w = self.tab_width(fonts, id);
                    tabs.push(Rect::from_xywh(x, strip.top(), w, strip_h));
                    x += w;
                }
                groups.push(GroupLayout {
                    path: path.clone(),
                    rect,
                    strip,
                    content,
                    tabs,
                    panels: panels.clone(),
                    active: *active,
                });
            }
            DockNode::Split {
                axis,
                children,
                sizes,
            } => {
                let n = children.len();
                if n == 0 {
                    return;
                }
                let gutters_len = (n.saturating_sub(1)) as f32 * GUTTER;
                let total = match axis {
                    DockAxis::Horizontal => rect.width(),
                    DockAxis::Vertical => rect.height(),
                } - gutters_len;
                let total = total.max(0.0);

                let sum: f32 = sizes.iter().sum();
                let mut cursor = match axis {
                    DockAxis::Horizontal => rect.left(),
                    DockAxis::Vertical => rect.top(),
                };
                for (i, child) in children.iter().enumerate() {
                    let frac = if sum > 0.0 { sizes[i] / sum } else { 1.0 / n as f32 };
                    let len = total * frac;
                    let child_rect = match axis {
                        DockAxis::Horizontal => {
                            Rect::from_xywh(cursor, rect.top(), len, rect.height())
                        }
                        DockAxis::Vertical => {
                            Rect::from_xywh(rect.left(), cursor, rect.width(), len)
                        }
                    };
                    path.push(i);
                    self.walk(child, child_rect, path, fonts, strip_h, groups, gutters);
                    path.pop();

                    cursor += len;
                    if i + 1 < n {
                        let g = match axis {
                            DockAxis::Horizontal => {
                                Rect::from_xywh(cursor, rect.top(), GUTTER, rect.height())
                            }
                            DockAxis::Vertical => {
                                Rect::from_xywh(rect.left(), cursor, rect.width(), GUTTER)
                            }
                        };
                        gutters.push(GutterLayout {
                            path: path.clone(),
                            index: i,
                            rect: g,
                            axis: *axis,
                        });
                        cursor += GUTTER;
                    }
                }
            }
        }
    }

    /// Ensure every registered panel appears somewhere in the tree (a panel
    /// registered but absent from a restored layout would otherwise vanish).
    fn adopt_orphans(&mut self) {
        let mut placed = Vec::new();
        self.root.collect_panels(&mut placed);
        let orphans: Vec<String> = self
            .panels
            .keys()
            .filter(|id| !placed.contains(*id))
            .cloned()
            .collect();
        if orphans.is_empty() {
            return;
        }
        if let Some(group) = first_tabs_mut(&mut self.root) {
            if let DockNode::Tabs { panels, .. } = group {
                for id in orphans {
                    panels.push(id);
                }
            }
        } else {
            self.root = DockNode::tabs(orphans);
        }
    }

    /// Drop the panels in the tree that have no registered content (a stale
    /// persisted layout referencing a panel the app no longer provides).
    fn drop_unknown(&mut self) {
        let known: Vec<String> = self.panels.keys().cloned().collect();
        retain_panels(&mut self.root, &known);
        self.root = prune(std::mem::replace(&mut self.root, DockNode::tabs(Vec::<String>::new())))
            .unwrap_or_else(|| DockNode::tabs(Vec::<String>::new()));
    }
}

// ---------------------------------------------------------------------------
// Tree helpers (pure, testable)
// ---------------------------------------------------------------------------

fn first_tabs_mut(node: &mut DockNode) -> Option<&mut DockNode> {
    match node {
        DockNode::Tabs { .. } => Some(node),
        DockNode::Split { children, .. } => children.iter_mut().find_map(first_tabs_mut),
    }
}

fn node_at_mut<'a>(node: &'a mut DockNode, path: &[usize]) -> Option<&'a mut DockNode> {
    match path.split_first() {
        None => Some(node),
        Some((i, rest)) => match node {
            DockNode::Split { children, .. } => {
                children.get_mut(*i).and_then(|c| node_at_mut(c, rest))
            }
            DockNode::Tabs { .. } => None,
        },
    }
}

fn retain_panels(node: &mut DockNode, known: &[String]) {
    match node {
        DockNode::Tabs { panels, active } => {
            panels.retain(|p| known.contains(p));
            if *active >= panels.len() {
                *active = panels.len().saturating_sub(1);
            }
        }
        DockNode::Split { children, .. } => {
            for c in children.iter_mut() {
                retain_panels(c, known);
            }
        }
    }
}

/// Remove empty tab groups and collapse single-child splits. Returns `None` if
/// the node itself is now empty.
fn prune(node: DockNode) -> Option<DockNode> {
    match node {
        DockNode::Tabs { panels, active } => {
            if panels.is_empty() {
                None
            } else {
                let active = active.min(panels.len() - 1);
                Some(DockNode::Tabs { panels, active })
            }
        }
        DockNode::Split {
            axis,
            children,
            sizes,
        } => {
            let mut kept: Vec<DockNode> = Vec::new();
            let mut kept_sizes: Vec<f32> = Vec::new();
            for (i, child) in children.into_iter().enumerate() {
                if let Some(c) = prune(c_into(child)) {
                    kept.push(c);
                    kept_sizes.push(*sizes.get(i).unwrap_or(&1.0));
                }
            }
            match kept.len() {
                0 => None,
                1 => Some(kept.into_iter().next().unwrap()),
                _ => {
                    let sum: f32 = kept_sizes.iter().sum();
                    let sizes = if sum > 0.0 {
                        kept_sizes.iter().map(|s| s / sum).collect()
                    } else {
                        vec![1.0 / kept.len() as f32; kept.len()]
                    };
                    Some(DockNode::Split {
                        axis,
                        children: kept,
                        sizes,
                    })
                }
            }
        }
    }
}

/// Identity — keeps `prune` readable when recursing over an owned child.
fn c_into(node: DockNode) -> DockNode {
    node
}

/// Remove `panel` from the tab group at `path` (no pruning, so paths stay valid).
fn remove_tab(root: &mut DockNode, path: &[usize], tab: usize) {
    if let Some(DockNode::Tabs { panels, active }) = node_at_mut(root, path) {
        if tab < panels.len() {
            panels.remove(tab);
            if *active >= panels.len() {
                *active = panels.len().saturating_sub(1);
            }
        }
    }
}

fn insert_tab(root: &mut DockNode, path: &[usize], index: usize, panel: String) {
    if let Some(DockNode::Tabs { panels, active }) = node_at_mut(root, path) {
        let index = index.min(panels.len());
        panels.insert(index, panel);
        *active = index;
    }
}

/// Replace the node at `path` with a split of [existing, new-tab] on `side`.
fn split_with(root: &mut DockNode, path: &[usize], side: Side, panel: String) {
    let Some(node) = node_at_mut(root, path) else {
        return;
    };
    let axis = match side {
        Side::Left | Side::Right => DockAxis::Horizontal,
        Side::Top | Side::Bottom => DockAxis::Vertical,
    };
    let existing = std::mem::replace(node, DockNode::tabs(Vec::<String>::new()));
    let fresh = DockNode::tabs(vec![panel]);
    let children = match side {
        Side::Left | Side::Top => vec![fresh, existing],
        Side::Right | Side::Bottom => vec![existing, fresh],
    };
    *node = DockNode::Split {
        axis,
        children,
        sizes: vec![0.5, 0.5],
    };
}

impl Widget for DockArea {
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

        // Absorb any panels a floating window handed back.
        for panel in take_redock() {
            self.panels.insert(panel.id.clone(), panel);
        }
        self.drop_unknown();
        self.adopt_orphans();

        let strip_h = self.strip_height(cx.fonts);
        let mut groups = Vec::new();
        let mut gutters = Vec::new();
        let mut path = Vec::new();
        let root = std::mem::replace(&mut self.root, DockNode::tabs(Vec::<String>::new()));
        self.walk(
            &root,
            Rect::from_xywh(0.0, 0.0, w, h),
            &mut path,
            cx.fonts,
            strip_h,
            &mut groups,
            &mut gutters,
        );
        self.root = root;
        self.groups = groups;
        self.gutters = gutters;

        // Lay out only the active panel of each group.
        let actives: Vec<(String, Size)> = self
            .groups
            .iter()
            .filter_map(|g| {
                g.panels
                    .get(g.active)
                    .map(|id| (id.clone(), g.content.size))
            })
            .collect();
        for (id, size) in actives {
            if let Some(panel) = self.panels.get_mut(&id) {
                panel.widget.layout(cx, Constraints::tight(size));
            }
        }

        constraints.constrain(Size::new(w, h))
    }

    fn paint(&mut self, cx: &mut PaintCx<'_>, bounds: Rect, scene: &mut Scene) {
        let p = &cx.theme.palette;
        let s = crate::text::scale();
        let line_h = cx.fonts.line_height(self.font_size, FontId::Ui);

        for (gi, group) in self.groups.iter().enumerate() {
            let strip = super::absolute(bounds, group.strip);
            scene.rect(strip, p.surface_variant);

            for (ti, tab_rel) in group.tabs.iter().enumerate() {
                let tab = super::absolute(bounds, *tab_rel);
                let id = &group.panels[ti];
                let Some(panel) = self.panels.get(id) else {
                    continue;
                };
                let active = ti == group.active;

                if active {
                    scene.push_rect(RectShape::fill(tab, p.surface));
                    scene.rect(
                        Rect::from_xywh(tab.left(), tab.top(), tab.width(), 2.0 * s),
                        p.accent,
                    );
                } else if self.hovered_tab == Some((gi, ti)) {
                    scene.push_rect(RectShape::fill(tab, p.hover));
                }

                let color = if active { p.text } else { p.text_muted };
                let ty = tab.top() + (tab.height() - line_h) * 0.5;
                let mut tx = tab.left() + 10.0 * s;
                if let Some(icon) = panel.icon {
                    scene.text_font(
                        Point::new(tx, ty),
                        icon.ch().to_string(),
                        self.font_size,
                        color,
                        icon.font_id(),
                    );
                    tx += cx
                        .fonts
                        .char_advance(icon.ch(), self.font_size, icon.font_id())
                        + 6.0 * s;
                }
                scene.text(
                    Point::new(tx, ty),
                    panel.title.clone(),
                    self.font_size,
                    color,
                );

                if panel.closable {
                    let close = close_rect(tab, s);
                    if self.hovered_close == Some((gi, ti)) {
                        scene.rounded_rect(close, p.active, cx.theme.radius.sm);
                    }
                    let x = glyphs::CROSS;
                    let cw = cx.fonts.char_advance(x.ch(), self.font_size - 1.0, x.font_id());
                    scene.text_font(
                        Point::new(
                            close.center().x - cw * 0.5,
                            close.top() + (close.height() - line_h) * 0.5,
                        ),
                        x.ch().to_string(),
                        self.font_size - 1.0,
                        if self.hovered_close == Some((gi, ti)) {
                            p.text
                        } else {
                            p.text_muted
                        },
                        x.font_id(),
                    );
                }
            }

            // Active panel content.
            let content = super::absolute(bounds, group.content);
            if let Some(id) = group.panels.get(group.active) {
                if let Some(panel) = self.panels.get_mut(id) {
                    scene.push_clip(content);
                    panel.widget.paint(cx, content, scene);
                    scene.pop_clip();
                }
            }
        }

        // Gutters.
        for (i, gutter) in self.gutters.iter().enumerate() {
            let abs = super::absolute(bounds, gutter.rect);
            let active = self.hovered_gutter == Some(i) || self.gutter_drag.is_some();
            let color = if active { p.accent } else { p.border };
            let handle = match gutter.axis {
                DockAxis::Horizontal => Rect::from_xywh(
                    abs.center().x - 1.0,
                    abs.top() + 6.0,
                    2.0,
                    (abs.height() - 12.0).max(0.0),
                ),
                DockAxis::Vertical => Rect::from_xywh(
                    abs.left() + 6.0,
                    abs.center().y - 1.0,
                    (abs.width() - 12.0).max(0.0),
                    2.0,
                ),
            };
            scene.rounded_rect(handle, color, 1.0);
        }

        // Drag feedback: the drop indicator, then a ghost following the cursor.
        if let Some(drag) = &self.drag {
            if drag.active {
                scene.begin_overlay();
                if let Some(target) = &drag.target {
                    if let Some(group) = self.groups.iter().find(|g| g.path == target.path) {
                        let rect = indicator_rect(
                            super::absolute(bounds, group.rect),
                            super::absolute(bounds, group.strip),
                            &target.zone,
                            group,
                            bounds,
                            s,
                        );
                        scene.push_rect(
                            RectShape::fill(rect, p.accent.with_alpha(0.25))
                                .with_corner_radius(cx.theme.radius.sm)
                                .with_border(2.0, p.accent),
                        );
                    }
                }
                if let Some(panel) = self.panels.get(&drag.panel) {
                    let w = self.tab_width(cx.fonts, &drag.panel);
                    let ghost = Rect::from_xywh(
                        drag.pos.x - w * 0.5,
                        drag.pos.y - 10.0 * s,
                        w,
                        self.strip_height(cx.fonts),
                    );
                    scene.push_rect(
                        RectShape::fill(ghost, p.surface.with_alpha(0.92))
                            .with_corner_radius(cx.theme.radius.sm)
                            .with_border(1.0, p.accent),
                    );
                    scene.text(
                        Point::new(ghost.left() + 10.0 * s, ghost.top() + 5.0 * s),
                        panel.title.clone(),
                        self.font_size,
                        p.text,
                    );
                }
                scene.end_overlay();
            }
        }

        self.context.paint(cx, scene);
    }

    fn event(&mut self, cx: &mut EventCx<'_>, bounds: Rect, event: &InputEvent) {
        // 1. Context menu floats above everything.
        if self.context.is_open() {
            if let Some(activation) = self.context.event(cx, event) {
                if let Some(id) = self.context_panel.clone() {
                    self.run_context_action(activation.index, &id);
                }
            }
            if cx.is_consumed() {
                return;
            }
        }

        // 2. An in-progress drag captures the pointer.
        if self.drag.is_some() {
            match event {
                InputEvent::PointerMoved { pos } => {
                    self.update_drag(bounds, *pos);
                    cx.consume();
                    return;
                }
                InputEvent::PointerReleased {
                    button: PointerButton::Primary,
                    ..
                } => {
                    self.finish_drag();
                    cx.consume();
                    return;
                }
                _ => {}
            }
        }
        if self.gutter_drag.is_some() {
            match event {
                InputEvent::PointerMoved { pos } => {
                    self.update_gutter_drag(bounds, *pos);
                    cx.consume();
                    return;
                }
                InputEvent::PointerReleased {
                    button: PointerButton::Primary,
                    ..
                } => {
                    self.gutter_drag = None;
                    cx.consume();
                    return;
                }
                _ => {}
            }
        }

        // 3. Panel content first (it may own popups floating over our chrome).
        let actives: Vec<(String, Rect)> = self
            .groups
            .iter()
            .filter_map(|g| {
                g.panels
                    .get(g.active)
                    .map(|id| (id.clone(), super::absolute(bounds, g.content)))
            })
            .collect();
        for (id, rect) in actives {
            let ev = cx.effective(event);
            if let Some(panel) = self.panels.get_mut(&id) {
                panel.widget.event(cx, rect, ev);
            }
        }
        if cx.is_consumed() {
            self.hovered_tab = None;
            self.hovered_close = None;
            return;
        }

        // 4. Our own chrome: tabs and gutters.
        match event {
            InputEvent::PointerMoved { pos } => {
                self.update_hover(bounds, *pos);
            }
            InputEvent::PointerLeft => {
                self.hovered_tab = None;
                self.hovered_close = None;
                self.hovered_gutter = None;
            }
            InputEvent::PointerPressed {
                pos,
                button: PointerButton::Primary,
            } => {
                // Gutter?
                if let Some(gi) = self.gutter_at(bounds, *pos) {
                    self.begin_gutter_drag(bounds, gi, *pos);
                    cx.consume();
                    return;
                }
                // Tab?
                if let Some((gi, ti)) = self.tab_at(bounds, *pos) {
                    let group = &self.groups[gi];
                    let tab = super::absolute(bounds, group.tabs[ti]);
                    let id = group.panels[ti].clone();
                    let closable = self.panels.get(&id).map(|p| p.closable).unwrap_or(false);

                    if closable && close_rect(tab, crate::text::scale()).contains(*pos) {
                        self.close_panel(&id);
                        cx.consume();
                        return;
                    }

                    // Activate, and arm a possible drag.
                    let path = group.path.clone();
                    if let Some(DockNode::Tabs { active, .. }) = node_at_mut(&mut self.root, &path) {
                        *active = ti;
                    }
                    self.drag = Some(TabDrag {
                        from: path,
                        tab: ti,
                        panel: id,
                        start: *pos,
                        pos: *pos,
                        active: false,
                        target: None,
                    });
                    cx.consume();
                }
            }
            InputEvent::PointerPressed {
                pos,
                button: PointerButton::Secondary,
            } => {
                if let Some((gi, ti)) = self.tab_at(bounds, *pos) {
                    let id = self.groups[gi].panels[ti].clone();
                    self.context_panel = Some(id);
                    self.context.open_at(*pos, context_items());
                    cx.consume();
                }
            }
            InputEvent::PointerReleased {
                button: PointerButton::Primary,
                ..
            } => {
                // A click without movement: just an activation, already done.
                self.drag = None;
            }
            _ => {}
        }
    }

    fn persist_save(&self, store: &mut crate::persist::Store) {
        if let Some(key) = &self.persist_key {
            store.set(key.clone(), &self.root);
        }
        for panel in self.panels.values() {
            panel.widget.persist_save(store);
        }
    }

    fn persist_restore(&mut self, store: &crate::persist::Store) {
        if let Some(key) = &self.persist_key {
            if let Some(root) = store.get::<DockNode>(key) {
                self.root = root;
                // A stale layout may name panels we no longer have, and may be
                // missing ones we do; both are repaired on the next layout.
            }
        }
        for panel in self.panels.values_mut() {
            panel.widget.persist_restore(store);
        }
    }
}

/// Menu shown when right-clicking a tab.
fn context_items() -> Vec<MenuItemSpec> {
    vec![
        MenuItemSpec::new("Close"),
        MenuItemSpec::new("Close Others"),
        MenuItemSpec::separator(),
        MenuItemSpec::new("Detach to Window").icon(glyphs::SQUARE),
        MenuItemSpec::separator(),
        MenuItemSpec::new("Split Right"),
        MenuItemSpec::new("Split Down"),
    ]
}

fn close_rect(tab: Rect, s: f32) -> Rect {
    let size = 14.0 * s;
    Rect::from_xywh(
        tab.right() - size - 5.0 * s,
        tab.center().y - size * 0.5,
        size,
        size,
    )
}

/// The highlight showing where a dragged tab would land.
fn indicator_rect(
    group_rect: Rect,
    strip: Rect,
    zone: &DropZone,
    group: &GroupLayout,
    bounds: Rect,
    s: f32,
) -> Rect {
    match zone {
        DropZone::Tab(index) => {
            let x = if let Some(tab) = group.tabs.get(*index) {
                super::absolute(bounds, *tab).left()
            } else if let Some(last) = group.tabs.last() {
                super::absolute(bounds, *last).right()
            } else {
                strip.left()
            };
            Rect::from_xywh(x - 1.5 * s, strip.top(), 3.0 * s, strip.height())
        }
        DropZone::Split(side) => {
            let half_w = group_rect.width() * 0.5;
            let half_h = group_rect.height() * 0.5;
            match side {
                Side::Left => {
                    Rect::from_xywh(group_rect.left(), group_rect.top(), half_w, group_rect.height())
                }
                Side::Right => Rect::from_xywh(
                    group_rect.left() + half_w,
                    group_rect.top(),
                    half_w,
                    group_rect.height(),
                ),
                Side::Top => {
                    Rect::from_xywh(group_rect.left(), group_rect.top(), group_rect.width(), half_h)
                }
                Side::Bottom => Rect::from_xywh(
                    group_rect.left(),
                    group_rect.top() + half_h,
                    group_rect.width(),
                    half_h,
                ),
            }
        }
    }
}

impl DockArea {
    fn tab_at(&self, bounds: Rect, pos: Point) -> Option<(usize, usize)> {
        for (gi, group) in self.groups.iter().enumerate() {
            for (ti, tab) in group.tabs.iter().enumerate() {
                if super::absolute(bounds, *tab).contains(pos) {
                    return Some((gi, ti));
                }
            }
        }
        None
    }

    fn gutter_at(&self, bounds: Rect, pos: Point) -> Option<usize> {
        self.gutters.iter().position(|g| {
            super::absolute(bounds, g.rect)
                .expand(Insets::all(3.0))
                .contains(pos)
        })
    }

    fn update_hover(&mut self, bounds: Rect, pos: Point) {
        self.hovered_gutter = self.gutter_at(bounds, pos);
        self.hovered_tab = self.tab_at(bounds, pos);
        self.hovered_close = None;
        if let Some((gi, ti)) = self.hovered_tab {
            let tab = super::absolute(bounds, self.groups[gi].tabs[ti]);
            if close_rect(tab, crate::text::scale()).contains(pos) {
                self.hovered_close = Some((gi, ti));
            }
        }
    }

    /// Which group is under `pos`, and where in it a drop would land.
    fn drop_target(&self, bounds: Rect, pos: Point) -> Option<DropTarget> {
        for group in &self.groups {
            let rect = super::absolute(bounds, group.rect);
            if !rect.contains(pos) {
                continue;
            }
            let strip = super::absolute(bounds, group.strip);
            if strip.contains(pos) {
                // Insert index = number of tabs whose midpoint is left of pos.
                let index = group
                    .tabs
                    .iter()
                    .filter(|t| super::absolute(bounds, **t).center().x < pos.x)
                    .count();
                return Some(DropTarget {
                    path: group.path.clone(),
                    zone: DropZone::Tab(index),
                });
            }

            let fx = (pos.x - rect.left()) / rect.width().max(1.0);
            let fy = (pos.y - rect.top()) / rect.height().max(1.0);
            let zone = if fx < EDGE {
                DropZone::Split(Side::Left)
            } else if fx > 1.0 - EDGE {
                DropZone::Split(Side::Right)
            } else if fy < EDGE {
                DropZone::Split(Side::Top)
            } else if fy > 1.0 - EDGE {
                DropZone::Split(Side::Bottom)
            } else {
                DropZone::Tab(group.panels.len())
            };
            return Some(DropTarget {
                path: group.path.clone(),
                zone,
            });
        }
        None
    }

    fn update_drag(&mut self, bounds: Rect, pos: Point) {
        let target = self.drop_target(bounds, pos);
        if let Some(drag) = &mut self.drag {
            drag.pos = pos;
            if !drag.active && (pos - drag.start).length() > DRAG_THRESHOLD {
                drag.active = true;
            }
            drag.target = if drag.active { target } else { None };
        }
    }

    /// Apply the drop. Mutation order matters: remove WITHOUT pruning first (so
    /// the target path stays valid), then insert/split, then normalize.
    fn finish_drag(&mut self) {
        let Some(drag) = self.drag.take() else {
            return;
        };
        if !drag.active {
            return; // a plain click
        }
        let Some(target) = drag.target else {
            return;
        };

        // Reorder inside the same strip is a pure vector move.
        if target.path == drag.from {
            if let DropZone::Tab(mut index) = target.zone {
                if let Some(DockNode::Tabs { panels, active }) =
                    node_at_mut(&mut self.root, &drag.from)
                {
                    if index > drag.tab {
                        index -= 1;
                    }
                    let index = index.min(panels.len().saturating_sub(1));
                    let id = panels.remove(drag.tab);
                    panels.insert(index, id);
                    *active = index;
                }
                return;
            }
        }

        remove_tab(&mut self.root, &drag.from, drag.tab);
        match target.zone {
            DropZone::Tab(index) => {
                insert_tab(&mut self.root, &target.path, index, drag.panel);
            }
            DropZone::Split(side) => {
                split_with(&mut self.root, &target.path, side, drag.panel);
            }
        }
        let root = std::mem::replace(&mut self.root, DockNode::tabs(Vec::<String>::new()));
        self.root = prune(root).unwrap_or_else(|| DockNode::tabs(Vec::<String>::new()));
    }

    fn begin_gutter_drag(&mut self, bounds: Rect, gi: usize, pos: Point) {
        let gutter = &self.gutters[gi];
        let path = gutter.path.clone();
        let index = gutter.index;
        let axis = gutter.axis;
        let (a, b, total) = {
            let split_rect = self.split_rect(bounds, &path);
            let Some(DockNode::Split { sizes, .. }) = node_at_mut(&mut self.root, &path) else {
                return;
            };
            let a = *sizes.get(index).unwrap_or(&0.5);
            let b = *sizes.get(index + 1).unwrap_or(&0.5);
            let total = match axis {
                DockAxis::Horizontal => split_rect.width(),
                DockAxis::Vertical => split_rect.height(),
            };
            (a, b, total.max(1.0))
        };

        self.gutter_drag = Some(GutterDrag {
            path,
            index,
            axis,
            start: pos,
            start_sizes: (a, b),
            total,
        });
    }

    /// The screen rect of the split at `path` (union of its groups).
    fn split_rect(&self, bounds: Rect, path: &[usize]) -> Rect {
        let mut union: Option<Rect> = None;
        for group in self.groups.iter().filter(|g| g.path.starts_with(path)) {
            let r = super::absolute(bounds, group.rect);
            union = Some(match union {
                None => r,
                Some(u) => Rect::from_min_max(
                    Point::new(u.left().min(r.left()), u.top().min(r.top())),
                    Point::new(u.right().max(r.right()), u.bottom().max(r.bottom())),
                ),
            });
        }
        union.unwrap_or(bounds)
    }

    fn update_gutter_drag(&mut self, _bounds: Rect, pos: Point) {
        let Some(drag) = &self.gutter_drag else {
            return;
        };
        let delta = match drag.axis {
            DockAxis::Horizontal => pos.x - drag.start.x,
            DockAxis::Vertical => pos.y - drag.start.y,
        };
        let frac = delta / drag.total;
        let (a0, b0) = drag.start_sizes;
        let pair = a0 + b0;
        let min = 0.08 * pair;
        let a = (a0 + frac).clamp(min, pair - min);
        let b = pair - a;

        let (path, index) = (drag.path.clone(), drag.index);
        if let Some(DockNode::Split { sizes, .. }) = node_at_mut(&mut self.root, &path) {
            if index + 1 < sizes.len() {
                sizes[index] = a;
                sizes[index + 1] = b;
            }
        }
    }

    /// Remove a panel from the layout tree AND the registry, handing back owned
    /// content. This is what makes detaching cheap: the widget simply moves.
    fn take_panel(&mut self, id: &str) -> Option<Panel> {
        let known: Vec<String> = self
            .panels
            .keys()
            .filter(|k| k.as_str() != id)
            .cloned()
            .collect();
        retain_panels(&mut self.root, &known);
        let root = std::mem::replace(&mut self.root, DockNode::tabs(Vec::<String>::new()));
        self.root = prune(root).unwrap_or_else(|| DockNode::tabs(Vec::<String>::new()));
        self.panels.remove(id)
    }

    fn close_panel(&mut self, id: &str) {
        self.take_panel(id);
    }

    /// Move a panel out of the dock and into its own OS window.
    fn detach_panel(&mut self, id: &str) {
        let Some(panel) = self.take_panel(id) else {
            return;
        };
        let title = format!("BaseUI — {}", panel.title);
        window::open(
            WindowSpec::new(title, FloatingPanel::new(panel))
                .size(560, 420)
                .context("panel"),
        );
    }

    fn run_context_action(&mut self, index: usize, id: &str) {
        match index {
            0 => self.close_panel(id),
            3 => self.detach_panel(id),
            1 => {
                // Close others in the same group.
                let group_panels = self
                    .groups
                    .iter()
                    .find(|g| g.panels.iter().any(|p| p == id))
                    .map(|g| g.panels.clone())
                    .unwrap_or_default();
                for other in group_panels {
                    if other != id {
                        self.close_panel(&other);
                    }
                }
            }
            5 | 6 => {
                let side = if index == 5 { Side::Right } else { Side::Bottom };
                let Some(group) = self
                    .groups
                    .iter()
                    .find(|g| g.panels.iter().any(|p| p == id))
                else {
                    return;
                };
                let path = group.path.clone();
                let tab = group.panels.iter().position(|p| p == id).unwrap_or(0);
                if group.panels.len() < 2 {
                    return; // splitting a lone tab off itself is a no-op
                }
                remove_tab(&mut self.root, &path, tab);
                split_with(&mut self.root, &path, side, id.to_string());
                let root =
                    std::mem::replace(&mut self.root, DockNode::tabs(Vec::<String>::new()));
                self.root = prune(root).unwrap_or_else(|| DockNode::tabs(Vec::<String>::new()));
            }
            _ => {}
        }
    }
}

// ---------------------------------------------------------------------------
// Detached (floating) panels
// ---------------------------------------------------------------------------

/// The root widget of a window holding a **detached** dock panel.
///
/// It owns the [`Panel`] outright — detaching *moves* the content rather than
/// sharing it, which is what the id-tree/registry split buys us. Clicking
/// **Dock** hands the panel back through the redock queue and closes the window;
/// the [`DockArea`] absorbs it on its next layout.
struct FloatingPanel {
    panel: Option<Panel>,
    font_size: f32,
    header_h: f32,
    dock_rect: Rect,
    hovered_dock: bool,
}

impl FloatingPanel {
    fn new(panel: Panel) -> Self {
        FloatingPanel {
            panel: Some(panel),
            font_size: 13.0,
            header_h: 30.0,
            dock_rect: Rect::ZERO,
            hovered_dock: false,
        }
    }

    fn content_rect(&self, bounds: Rect) -> Rect {
        Rect::from_xywh(
            bounds.left(),
            bounds.top() + self.header_h,
            bounds.width(),
            (bounds.height() - self.header_h).max(0.0),
        )
    }
}

impl Widget for FloatingPanel {
    fn layout(&mut self, cx: &mut LayoutCx<'_>, constraints: Constraints) -> Size {
        // A "Dock Panel Back" command (palette / Ctrl+D) targets a window, not a
        // widget, so the request is consumed here.
        if take_redock_request(cx.window) {
            if let Some(panel) = self.panel.take() {
                queue_redock(panel);
            }
            if let Some(id) = cx.window {
                window::close(id);
            }
        }

        let s = crate::text::scale();
        let line_h = cx.fonts.line_height(self.font_size, FontId::Ui);
        self.header_h = line_h + 12.0 * s;

        let w = if constraints.max.width.is_finite() {
            constraints.max.width
        } else {
            560.0
        };
        let h = if constraints.max.height.is_finite() {
            constraints.max.height
        } else {
            420.0
        };

        let label_w = cx.fonts.measure("Dock", self.font_size, FontId::Ui).width;
        let btn_w = label_w + 18.0 * s;
        self.dock_rect = Rect::from_xywh(
            w - btn_w - 8.0 * s,
            (self.header_h - line_h - 6.0 * s) * 0.5,
            btn_w,
            line_h + 6.0 * s,
        );

        if let Some(panel) = self.panel.as_mut() {
            panel.widget.layout(
                cx,
                Constraints::tight(Size::new(w, (h - self.header_h).max(0.0))),
            );
        }
        constraints.constrain(Size::new(w, h))
    }

    fn paint(&mut self, cx: &mut PaintCx<'_>, bounds: Rect, scene: &mut Scene) {
        let p = &cx.theme.palette;
        let s = crate::text::scale();
        let line_h = cx.fonts.line_height(self.font_size, FontId::Ui);

        let header = Rect::from_xywh(bounds.left(), bounds.top(), bounds.width(), self.header_h);
        scene.rect(header, p.surface_variant);

        let ty = header.top() + (header.height() - line_h) * 0.5;
        let mut tx = header.left() + 10.0 * s;
        if let Some(panel) = self.panel.as_ref() {
            if let Some(icon) = panel.icon {
                scene.text_font(
                    Point::new(tx, ty),
                    icon.ch().to_string(),
                    self.font_size,
                    p.text,
                    icon.font_id(),
                );
                tx += cx
                    .fonts
                    .char_advance(icon.ch(), self.font_size, icon.font_id())
                    + 6.0 * s;
            }
            scene.text(
                Point::new(tx, ty),
                panel.title.clone(),
                self.font_size,
                p.text,
            );
        }

        // "Dock" button — the redock affordance.
        let btn = super::absolute(bounds, self.dock_rect);
        scene.push_rect(
            RectShape::fill(btn, if self.hovered_dock { p.accent } else { p.surface })
                .with_corner_radius(cx.theme.radius.sm)
                .with_border(1.0, p.border),
        );
        let lw = cx.fonts.measure("Dock", self.font_size, FontId::Ui).width;
        scene.text(
            Point::new(
                btn.center().x - lw * 0.5,
                btn.top() + (btn.height() - line_h) * 0.5,
            ),
            "Dock",
            self.font_size,
            if self.hovered_dock { p.on_accent } else { p.text },
        );

        let content = self.content_rect(bounds);
        if let Some(panel) = self.panel.as_mut() {
            scene.push_clip(content);
            panel.widget.paint(cx, content, scene);
            scene.pop_clip();
        }
    }

    fn event(&mut self, cx: &mut EventCx<'_>, bounds: Rect, event: &InputEvent) {
        let btn = super::absolute(bounds, self.dock_rect);
        match event {
            InputEvent::PointerMoved { pos } => self.hovered_dock = btn.contains(*pos),
            InputEvent::PointerLeft => self.hovered_dock = false,
            InputEvent::PointerPressed {
                pos,
                button: PointerButton::Primary,
            } if btn.contains(*pos) => {
                // Hand the panel back and close this window; the DockArea picks
                // it up on its next layout.
                if let Some(panel) = self.panel.take() {
                    queue_redock(panel);
                }
                if let Some(id) = cx.window {
                    window::close(id);
                }
                cx.consume();
                return;
            }
            _ => {}
        }

        let content = self.content_rect(bounds);
        if let Some(panel) = self.panel.as_mut() {
            panel.widget.event(cx, content, event);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Null;
    impl Widget for Null {
        fn layout(&mut self, _cx: &mut LayoutCx<'_>, c: Constraints) -> Size {
            c.constrain(Size::ZERO)
        }
        fn paint(&mut self, _cx: &mut PaintCx<'_>, _b: Rect, _s: &mut Scene) {}
    }

    /// Detaching MOVES the panel out (id tree + registry), and redocking hands
    /// the same owned content back. This is the whole point of separating layout
    /// from content: no widget is ever re-parented behind anyone's back.
    #[test]
    fn detaching_takes_ownership_and_redock_returns_it() {
        let mut dock = DockArea::new(DockNode::split(
            DockAxis::Horizontal,
            vec![DockNode::tabs(["a"]), DockNode::tabs(["b"])],
        ))
        .panel(Panel::new("a", "A", Null))
        .panel(Panel::new("b", "B", Null));

        let taken = dock.take_panel("a").expect("panel a should be detachable");
        assert_eq!(taken.id, "a");
        assert!(!dock.panels.contains_key("a"), "content left the registry");
        // Its group is now empty -> pruned; the split collapses to the survivor.
        assert_eq!(ids(&dock.root), vec!["b"]);

        // Redocking hands the same owned panel back.
        queue_redock(taken);
        let returned = take_redock();
        assert_eq!(returned.len(), 1);
        assert_eq!(returned[0].id, "a");
    }

    fn ids(node: &DockNode) -> Vec<String> {
        let mut out = Vec::new();
        node.collect_panels(&mut out);
        out
    }

    #[test]
    fn prune_removes_empty_tabs_and_collapses_single_child_splits() {
        let tree = DockNode::Split {
            axis: DockAxis::Horizontal,
            children: vec![
                DockNode::tabs(Vec::<String>::new()), // empty -> dropped
                DockNode::tabs(vec!["a"]),
            ],
            sizes: vec![0.5, 0.5],
        };
        let pruned = prune(tree).unwrap();
        // The split had one surviving child, so it collapses into that child.
        assert!(matches!(pruned, DockNode::Tabs { .. }));
        assert_eq!(ids(&pruned), vec!["a"]);
    }

    #[test]
    fn split_with_wraps_the_target_and_keeps_both() {
        let mut root = DockNode::tabs(vec!["a", "b"]);
        split_with(&mut root, &[], Side::Right, "c".into());
        match &root {
            DockNode::Split { axis, children, .. } => {
                assert_eq!(*axis, DockAxis::Horizontal);
                assert_eq!(children.len(), 2);
                // Right side => existing first, new second.
                assert_eq!(ids(&children[0]), vec!["a", "b"]);
                assert_eq!(ids(&children[1]), vec!["c"]);
            }
            _ => panic!("expected a split"),
        }
    }

    #[test]
    fn moving_the_last_tab_out_of_a_group_prunes_it() {
        // [ tabs(a) | tabs(b) ]  -- drag `a` into b's strip
        let mut root = DockNode::Split {
            axis: DockAxis::Horizontal,
            children: vec![DockNode::tabs(vec!["a"]), DockNode::tabs(vec!["b"])],
            sizes: vec![0.5, 0.5],
        };
        remove_tab(&mut root, &[0], 0);
        insert_tab(&mut root, &[1], 1, "a".into());
        let root = prune(root).unwrap();

        // Group 0 is now empty -> pruned; the split collapses to the single group.
        assert!(matches!(root, DockNode::Tabs { .. }));
        assert_eq!(ids(&root), vec!["b", "a"]);
    }

    #[test]
    fn layout_tree_round_trips_through_json() {
        let tree = DockNode::Split {
            axis: DockAxis::Vertical,
            children: vec![DockNode::tabs(vec!["a", "b"]), DockNode::tabs(vec!["c"])],
            sizes: vec![0.6, 0.4],
        };
        let json = serde_json::to_string(&tree).unwrap();
        let back: DockNode = serde_json::from_str(&json).unwrap();
        assert_eq!(ids(&back), vec!["a", "b", "c"]);
    }
}
