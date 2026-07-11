//! BaseUI full app-frame demo (milestone M6).
//!
//! A complete application shell assembled from BaseUI widgets:
//! - **MenuBar** with dropdown menus (File / Edit / View / Help).
//! - **Toolbar** with real **font-gis** icons, toggles, and a flexible spacer.
//! - A resizable **Split**: fixed Outliner ([`TreeView`] with visibility/render
//!   toggle icons) | flexible content viewport | fixed **tabbed** inspector.
//! - **StatusBar** whose items react to selection / the last action.
//!
//! Drag the gutters between the three center panes; open the menus; toggle the
//! toolbar buttons; click a tree row to update the viewport and status bar.
//!
//! ```text
//! cargo run -p inspector
//! ```

use baseui::icon::{gis, glyphs};
use baseui::layout::Constraints;
use baseui::paint::Scene;
use baseui::widget::{
    DragValue, LayoutCx, Menu, MenuBar, PaintCx, PropGroup, PropertyView, ScrollArea, Slider,
    Split, StatusBar, StatusItem, TabView, Toolbar, TreeNode, TreeView, Widget,
};
use baseui::{App, Color, Point, Rect, Signal, Size};

fn c(r: u8, g: u8, b: u8) -> Color {
    Color::rgb8(r, g, b)
}

/// The flexible middle content panel; shows the selection and its live size.
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
        let frame = bounds.shrink(baseui::Insets::all(16.0));
        scene.stroke_rect(frame, p.border, 1.0, cx.theme.radius.md);

        let name = self.selected.get();
        let ts = cx.fonts.measure(&name, 30.0, baseui::text::FontId::Ui);
        scene.text(
            Point::new(frame.center().x - ts.width * 0.5, frame.center().y - ts.height * 0.5 - 12.0),
            name,
            30.0,
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

fn sig(v: f32) -> Signal<f32> {
    baseui::core::create_signal(v)
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    let selected = baseui::core::create_signal(String::from("Cube"));
    let last_action = baseui::core::create_signal(String::from("Ready"));
    let grid_on = baseui::core::create_signal(true);
    let snap_on = baseui::core::create_signal(false);
    let zoom = sig(1.0);

    // --- Menu bar ---------------------------------------------------------
    let act = |s: &'static str| move || last_action.set(s.to_string());
    let menubar = MenuBar::new()
        .menu(
            Menu::new("File")
                .item("New", act("File ▸ New"))
                .item("Open…", act("File ▸ Open"))
                .item("Save", act("File ▸ Save"))
                .separator()
                .item("Quit", act("File ▸ Quit")),
        )
        .menu(
            Menu::new("Edit")
                .item("Undo", act("Edit ▸ Undo"))
                .item("Redo", act("Edit ▸ Redo"))
                .separator()
                .item("Preferences…", act("Edit ▸ Preferences")),
        )
        .menu(
            Menu::new("View")
                .item("Zoom In", act("View ▸ Zoom In"))
                .item("Zoom Out", act("View ▸ Zoom Out"))
                .item("Reset", act("View ▸ Reset")),
        )
        .menu(Menu::new("Help").item("About BaseUI", act("Help ▸ About")));

    // --- Toolbar (real font-gis icons) ------------------------------------
    let toolbar = Toolbar::new()
        .button_icon(gis::MAP, act("Map"))
        .button_icon(gis::LAYERS, act("Layers"))
        .button_icon(gis::GLOBE, act("Globe"))
        .separator()
        .button_icon(gis::POINT, act("Point"))
        .button_icon(gis::POLYGON, act("Polygon"))
        .button_icon(gis::MEASURE, act("Measure"))
        .button_icon(gis::COMPASS, act("Compass"))
        .separator()
        .toggle_icon(gis::LAYER, grid_on)
        .toggle_icon(gis::MOVE, snap_on)
        .spacer()
        .button_labeled(gis::MAP_OPTIONS, "Options", act("Options"));

    // --- Outliner ---------------------------------------------------------
    let camera = c(0x6c, 0xc6, 0x8a);
    let mesh = c(0xe0, 0x8a, 0x3c);
    let light = c(0xe6, 0xc2, 0x4e);
    let eye_on = c(0xd8, 0xd8, 0xde);
    let render_on = c(0x4d, 0x9c, 0xf5);
    let obj = |name: &str, col: Color, vis: bool, rend: bool| {
        TreeNode::leaf(name)
            .icon_color(col)
            .action(glyphs::EYE, eye_on, vis)
            .action(glyphs::DIAMOND, render_on, rend)
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

    // --- Tabbed inspector -------------------------------------------------
    let location = [sig(0.0), sig(0.0), sig(0.0)];
    let rotation = [sig(0.0), sig(0.0), sig(0.0)];
    let scale = [sig(1.0), sig(1.0), sig(1.0)];
    let fov = sig(50.0);
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
    let (mr, mg, mb, rough) = (sig(0.8), sig(0.3), sig(0.2), sig(0.4));
    let material_tab = ScrollArea::new(
        PropertyView::new().group(
            PropGroup::new("Surface")
                .icon_color(mesh)
                .row("Base R", Slider::new(mr).range(0.0, 1.0).width(180.0))
                .row("Base G", Slider::new(mg).range(0.0, 1.0).width(180.0))
                .row("Base B", Slider::new(mb).range(0.0, 1.0).width(180.0))
                .row("Roughness", Slider::new(rough).range(0.0, 1.0).width(180.0)),
        ),
    );
    let inspector = TabView::new()
        .tab_icon(glyphs::GEAR, "Object", object_tab)
        .tab("Material", material_tab);

    // --- Center split -----------------------------------------------------
    let center = Split::horizontal()
        .fixed_range(260.0, 160.0, 420.0, ScrollArea::new(tree))
        .flex(Viewport { selected })
        .fixed_range(360.0, 260.0, 560.0, inspector);

    // --- Status bar -------------------------------------------------------
    let status = StatusBar::new()
        .item(StatusItem::dynamic(move || last_action.get()).icon(glyphs::CHECK).color(c(0x5c, 0xc9, 0x7a)))
        .item(StatusItem::dynamic(move || format!("Selection: {}", selected.get())))
        .item(StatusItem::new("font-gis ✓").color(c(0x62, 0xb6, 0xd6)).right())
        .item(StatusItem::dynamic(move || format!("Zoom {:.0}%", zoom.get() * 100.0)).right())
        .item(StatusItem::new("BaseUI M6").right());

    // --- Frame: menu / toolbar / center / status (vertical, no gutters) ---
    let root = Split::vertical()
        .gutter(0.0)
        .fixed_range(30.0, 30.0, 30.0, menubar)
        .fixed_range(40.0, 40.0, 40.0, toolbar)
        .flex(center)
        .fixed_range(26.0, 26.0, 26.0, status);

    App::new()
        .with_title("BaseUI — App Frame")
        .with_size(1200, 780)
        .with_root(root)
        .run()
}
