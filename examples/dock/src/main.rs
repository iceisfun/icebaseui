//! BaseUI docking demo.
//!
//! A workspace of dockable panels:
//!
//! - **drag a tab** to reorder it within its strip, move it to another group, or
//!   drop it on a group's edge to **split** (a live indicator shows where it will
//!   land),
//! - **right-click a tab** for Close / Close Others / Split Right / Split Down,
//! - **× closes** a tab, **drag the gutters** to resize,
//! - the whole layout **persists** between runs.
//!
//! The panels are ordinary widgets (a tree, a property inspector, a hex viewer);
//! the dock only ever moves their **ids** around.
//!
//! ```text
//! cargo run -p dock
//! ```

use baseui::command::{self, CommandMeta};
use baseui::icon::{gis, glyphs};
use baseui::layout::Constraints;
use baseui::paint::Scene;
use baseui::widget::{
    DockArea, DockAxis, DockNode, DragValue, HexView, LayoutCx, Menu, MenuBar, PaintCx, Panel,
    PropGroup, PropertyView, ScrollArea, Slider, Split, StatusBar, StatusItem, TreeNode, TreeView,
    Widget,
};
use baseui::{App, Color, Point, Rect, Signal, Size};

fn col(r: u8, g: u8, b: u8) -> Color {
    Color::rgb8(r, g, b)
}

fn sig(v: f32) -> Signal<f32> {
    baseui::core::create_signal(v)
}

/// A filling placeholder panel that reports its own size.
struct Viewport {
    label: String,
    tint: Color,
}

impl Widget for Viewport {
    fn layout(&mut self, _cx: &mut LayoutCx<'_>, c: Constraints) -> Size {
        c.constrain(c.max)
    }

    fn paint(&mut self, cx: &mut PaintCx<'_>, bounds: Rect, scene: &mut Scene) {
        let p = &cx.theme.palette;
        scene.rect(bounds, p.background.lerp(p.surface, 0.4));
        let frame = bounds.shrink(baseui::Insets::all(10.0));
        scene.stroke_rect(frame, self.tint, 1.0, cx.theme.radius.md);

        let ts = cx
            .fonts
            .measure(&self.label, 24.0, baseui::text::FontId::Ui);
        scene.text(
            Point::new(
                frame.center().x - ts.width * 0.5,
                frame.center().y - ts.height * 0.5 - 10.0,
            ),
            self.label.clone(),
            24.0,
            self.tint,
        );
        let sub = format!("{:.0} × {:.0}", bounds.width(), bounds.height());
        let sw = cx.fonts.measure(&sub, 12.0, baseui::text::FontId::Ui);
        scene.text(
            Point::new(frame.center().x - sw.width * 0.5, frame.center().y + 18.0),
            sub,
            12.0,
            p.text_muted,
        );
    }
}

fn sample_bytes() -> Vec<u8> {
    let mut data = Vec::new();
    data.extend_from_slice(b"BaseUI dock demo\x00\x01\x02");
    data.extend(0u8..=255u8);
    data.extend_from_slice(b"drag a tab to reorder, split, or regroup");
    data
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    let blue = col(0x4d, 0x9c, 0xf5);
    let orange = col(0xe0, 0x8a, 0x3c);
    let green = col(0x6c, 0xc6, 0x8a);
    let purple = col(0xc7, 0x6c, 0xd6);

    // --- Panel content: ordinary widgets ----------------------------------
    let outliner = ScrollArea::new(TreeView::new(vec![
        TreeNode::branch(
            "Scene Collection",
            vec![
                TreeNode::leaf("Camera")
                    .icon(gis::COMPASS)
                    .icon_color(green),
                TreeNode::leaf("Cube").icon(gis::POLYGON).icon_color(orange),
                TreeNode::leaf("Light")
                    .icon(glyphs::STAR)
                    .icon_color(col(0xe6, 0xc2, 0x4e)),
            ],
        )
        .icon(gis::GLOBE)
        .icon_color(blue),
    ]));

    let (px, py, pz) = (sig(0.0), sig(0.0), sig(0.0));
    let rough = sig(0.4);
    let properties = ScrollArea::new(
        PropertyView::new()
            .group(
                PropGroup::new("Transform")
                    .icon_color(orange)
                    .row("X", DragValue::new(px).speed(0.01).decimals(3))
                    .row("Y", DragValue::new(py).speed(0.01).decimals(3))
                    .row("Z", DragValue::new(pz).speed(0.01).decimals(3)),
            )
            .group(
                PropGroup::new("Surface")
                    .icon_color(purple)
                    .row("Roughness", Slider::new(rough).range(0.0, 1.0).width(160.0)),
            )
            .persist("dock.props"),
    );

    // --- Dock layout: ids only --------------------------------------------
    //   [ outliner | [ (viewport, hex) / console ] | properties ]
    let layout = DockNode::split(
        DockAxis::Horizontal,
        vec![
            DockNode::tabs(["outliner"]),
            DockNode::split(
                DockAxis::Vertical,
                vec![
                    DockNode::tabs(["viewport", "hex"]),
                    DockNode::tabs(["console"]),
                ],
            ),
            DockNode::tabs(["properties"]),
        ],
    );

    let dock = DockArea::new(layout)
        .panel(
            Panel::new(
                "viewport",
                "Viewport",
                Viewport {
                    label: "Viewport".into(),
                    tint: blue,
                },
            )
            .icon(gis::MAP)
            .pinned(),
        )
        .panel(Panel::new("outliner", "Outliner", outliner).icon(gis::LAYERS))
        .panel(Panel::new("properties", "Properties", properties).icon(glyphs::GEAR))
        .panel(
            Panel::new(
                "hex",
                "Hex",
                HexView::new(sample_bytes()).rows(20).font_size(12.0),
            )
            .icon(gis::POINT),
        )
        .panel(
            Panel::new(
                "console",
                "Console",
                Viewport {
                    label: "Console".into(),
                    tint: green,
                },
            )
            .icon(gis::MEASURE),
        )
        .persist("dock.layout");

    // --- Global commands (the main window's palette; the dock registers its own
    //     panel-scoped ones, which only appear in a detached panel window) -----
    command::register(
        CommandMeta::new("view.text.inc", "Increase Text Size")
            .category("View")
            .icon(glyphs::CIRCLE)
            .color(blue)
            .shortcut("Ctrl+="),
        || baseui::text::set_scale(baseui::text::scale() + 0.1),
    );
    command::register(
        CommandMeta::new("view.text.dec", "Decrease Text Size")
            .category("View")
            .icon(glyphs::CIRCLE_OUTLINE)
            .color(blue)
            .shortcut("Ctrl+-"),
        || baseui::text::set_scale(baseui::text::scale() - 0.1),
    );
    command::register(
        CommandMeta::new("view.text.reset", "Reset Text Size")
            .category("View")
            .icon(glyphs::DOT)
            .color(blue)
            .shortcut("Ctrl+0"),
        || baseui::text::set_scale(1.0),
    );

    // --- App frame around the dock ----------------------------------------
    let menubar = MenuBar::new()
        .menu(Menu::new("File").item("Quit", || {}))
        .menu(
            Menu::new("View")
                .item_icon(glyphs::CIRCLE, "Increase Text Size", || {
                    command::run("view.text.inc")
                })
                .item_icon(glyphs::CIRCLE_OUTLINE, "Decrease Text Size", || {
                    command::run("view.text.dec")
                })
                .item_icon(glyphs::DOT, "Reset Text Size", || {
                    command::run("view.text.reset")
                }),
        )
        .menu(Menu::new("Help").item("Drag a tab to reorder / split", || {}));

    let status = StatusBar::new()
        .item(
            StatusItem::new("Drag tabs · right-click for menu · drag gutters")
                .icon(glyphs::CHECK)
                .color(green),
        )
        .item(StatusItem::new("Layout persists").right().color(blue));

    let root = Split::vertical()
        .gutter(0.0)
        .fixed_range(30.0, 30.0, 30.0, menubar)
        .flex(dock)
        .fixed_range(26.0, 26.0, 26.0, status);

    App::new()
        .with_title("BaseUI — Docking")
        .with_size(1180, 760)
        .with_persistence(std::env::temp_dir().join("baseui-dock-state.json"))
        .with_root(root)
        .run()
}
