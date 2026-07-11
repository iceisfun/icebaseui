//! BaseUI app-frame demo: a resizable [`Split`] with a fixed **Outliner**
//! (a [`TreeView`] with right-floating visibility/render toggle icons), a
//! **flexible content viewport** that widens with the window, and a fixed,
//! **tabbed** property inspector on the right. Drag the gutters to resize; the
//! middle absorbs the change.
//!
//! ```text
//! cargo run -p inspector
//! ```

use baseui::icon::{Icon, glyphs};
use baseui::layout::Constraints;
use baseui::paint::Scene;
use baseui::widget::{
    DragValue, LayoutCx, PaintCx, PropGroup, PropertyView, ScrollArea, Slider, Split, TabView,
    TreeNode, TreeView, Widget,
};
use baseui::{App, Color, Point, Rect, Signal, Size};

fn c(r: u8, g: u8, b: u8) -> Color {
    Color::rgb8(r, g, b)
}

/// The flexible middle "content" panel. Fills its pane and shows the selected
/// object plus its live size, so resizing the split is visible.
struct Viewport {
    selected: Signal<String>,
}

impl Widget for Viewport {
    fn layout(&mut self, _cx: &mut LayoutCx<'_>, constraints: Constraints) -> Size {
        constraints.constrain(constraints.max)
    }

    fn paint(&mut self, cx: &mut PaintCx<'_>, bounds: Rect, scene: &mut Scene) {
        let p = &cx.theme.palette;
        scene.rect(bounds, p.background.lerp(p.surface, 0.5));

        // A faint framed viewport region.
        let frame = bounds.shrink(baseui::Insets::all(16.0));
        scene.stroke_rect(frame, p.border, 1.0, cx.theme.radius.md);

        let name = self.selected.get();
        let title_size = 30.0;
        let tw = cx.fonts.measure(&name, title_size, baseui::text::FontId::Ui);
        scene.text(
            Point::new(
                frame.center().x - tw.width * 0.5,
                frame.center().y - tw.height * 0.5 - 12.0,
            ),
            name,
            title_size,
            p.text,
        );

        let sub = format!("content viewport — {:.0} × {:.0}", bounds.width(), bounds.height());
        let sw = cx.fonts.measure(&sub, 13.0, baseui::text::FontId::Ui);
        scene.text(
            Point::new(frame.center().x - sw.width * 0.5, frame.center().y + 20.0),
            sub,
            13.0,
            p.text_muted,
        );
    }
}

fn xyz(title: &str, icon: Color, s: [Signal<f32>; 3], speed: f32) -> PropGroup {
    let [x, y, z] = s;
    PropGroup::new(title.to_string())
        .icon_color(icon)
        .row("X", DragValue::new(x).speed(speed).decimals(3))
        .row("Y", DragValue::new(y).speed(speed).decimals(3))
        .row("Z", DragValue::new(z).speed(speed).decimals(3))
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    let selected = baseui::core::create_signal(String::from("Cube"));

    // --- Outliner (with per-row toggle icons) -----------------------------
    let camera = c(0x6c, 0xc6, 0x8a);
    let mesh = c(0xe0, 0x8a, 0x3c);
    let light = c(0xe6, 0xc2, 0x4e);
    let eye_on = c(0xd8, 0xd8, 0xde);
    let render_on = c(0x4d, 0x9c, 0xf5);

    let obj = |name: &str, col: Color, visible: bool, render: bool| {
        TreeNode::leaf(name)
            .icon_color(col)
            .action(Icon::glyph(glyphs::EYE), eye_on, visible)
            .action(Icon::glyph(glyphs::DIAMOND), render_on, render)
    };

    let tree = TreeView::new(vec![TreeNode::branch(
        "Scene Collection",
        vec![TreeNode::branch(
            "Collection",
            vec![
                obj("Camera", camera, true, true),
                obj("Cube", mesh, true, true),
                obj("Light", light, true, false),
                obj("Sphere", mesh, false, true),
            ],
        )],
    )])
    .on_select(move |label| selected.set(label.to_string()));

    // --- Tabbed property inspector ----------------------------------------
    let location = [
        baseui::core::create_signal(0.0),
        baseui::core::create_signal(0.0),
        baseui::core::create_signal(0.0),
    ];
    let rotation = [
        baseui::core::create_signal(0.0),
        baseui::core::create_signal(0.0),
        baseui::core::create_signal(0.0),
    ];
    let scale = [
        baseui::core::create_signal(1.0),
        baseui::core::create_signal(1.0),
        baseui::core::create_signal(1.0),
    ];
    let fov = baseui::core::create_signal(50.0f32);

    let object_tab = ScrollArea::new(
        PropertyView::new()
            .group(xyz("Location", mesh, location, 0.01))
            .group(xyz("Rotation", c(0x8a, 0x8a, 0xf0), rotation, 0.5))
            .group(xyz("Scale", camera, scale, 0.01))
            .group(
                PropGroup::new("Camera")
                    .icon_color(camera)
                    .row("FOV", DragValue::new(fov).range(1.0, 179.0).speed(0.25).decimals(1)),
            ),
    );

    let subdiv = baseui::core::create_signal(2.0f32);
    let bevel = baseui::core::create_signal(0.1f32);
    let modifiers_tab = ScrollArea::new(
        PropertyView::new()
            .group(
                PropGroup::new("Subdivision")
                    .icon_color(c(0x62, 0xb6, 0xd6))
                    .row("Levels", Slider::new(subdiv).range(0.0, 6.0).width(180.0))
                    .row("Render", DragValue::new(subdiv).range(0.0, 6.0).speed(0.05).decimals(0)),
            )
            .group(
                PropGroup::new("Bevel")
                    .icon_color(c(0xc7, 0x6c, 0xd6))
                    .row("Amount", DragValue::new(bevel).range(0.0, 1.0).speed(0.005).decimals(3)),
            ),
    );

    let (mr, mg, mb) = (
        baseui::core::create_signal(0.8f32),
        baseui::core::create_signal(0.3f32),
        baseui::core::create_signal(0.2f32),
    );
    let rough = baseui::core::create_signal(0.4f32);
    let material_tab = ScrollArea::new(
        PropertyView::new().group(
            PropGroup::new("Surface")
                .icon_color(c(0xe0, 0x8a, 0x3c))
                .row("Base R", Slider::new(mr).range(0.0, 1.0).width(180.0))
                .row("Base G", Slider::new(mg).range(0.0, 1.0).width(180.0))
                .row("Base B", Slider::new(mb).range(0.0, 1.0).width(180.0))
                .row("Roughness", Slider::new(rough).range(0.0, 1.0).width(180.0)),
        ),
    );

    let inspector = TabView::new()
        .tab_icon(Icon::glyph(glyphs::GEAR), "Object", object_tab)
        .tab("Modifiers", modifiers_tab)
        .tab("Material", material_tab);

    // --- Resizable app frame ---------------------------------------------
    let root = Split::horizontal()
        .fixed_range(260.0, 160.0, 420.0, ScrollArea::new(tree))
        .flex(Viewport { selected })
        .fixed_range(380.0, 260.0, 560.0, inspector);

    App::new()
        .with_title("BaseUI — Inspector")
        .with_size(1180, 760)
        .with_root(root)
        .run()
}
