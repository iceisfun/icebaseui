//! BaseUI application-shell demo (M6 + M7).
//!
//! A full app frame — MenuBar, Toolbar, resizable Split (Outliner | Viewport |
//! tabbed Inspector), StatusBar — plus the M7 **command system**:
//!
//! - Commands are registered once (id, title, category, font-gis icon, color,
//!   shortcut) and invoked from menus, the toolbar, keyboard shortcuts, and the
//!   **Command Palette**.
//! - Press **F1** (or Ctrl+Shift+P) for the palette; type to filter, ↑/↓ to
//!   move, Enter to run, Esc to close.
//! - The **Create** menu shows Maya-style options gears (Cube/Sphere/Plane ⚙).
//! - Tree nodes use real font-gis type icons; each row has visibility/render
//!   toggle icons.
//!
//! ```text
//! cargo run -p inspector
//! ```

use baseui::bus;
use baseui::command::{self, CommandMeta};
use baseui::icon::{gis, glyphs};
use baseui::layout::Constraints;
use baseui::paint::Scene;
use baseui::widget::{
    Column, ComboBox, DragValue, Label, LayoutCx, Menu, MenuBar, MenuItemSpec, PaintCx, PropGroup,
    PropertyView, ScrollArea, Slider, Split, StatusBar, StatusItem, TabView, TextBox, Toolbar,
    TreeNode, TreeView, Widget,
};
use baseui::window::{self, WindowSpec};
use baseui::{App, Color, Icon, Point, Rect, Signal, Size};

fn col(r: u8, g: u8, b: u8) -> Color {
    Color::rgb8(r, g, b)
}

/// An event-bus message: the outliner selection changed. Published by the tree,
/// consumed by whoever cares — no direct reference between them.
struct SelectionChanged {
    name: String,
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

/// Register a command whose handler records its title into `last_action`.
fn cmd(
    last_action: Signal<String>,
    id: &str,
    title: &str,
    category: &str,
    icon: Icon,
    color: Color,
    shortcut: Option<&str>,
) {
    let mut meta = CommandMeta::new(id, title).category(category).icon(icon).color(color);
    if let Some(sc) = shortcut {
        meta = meta.shortcut(sc);
    }
    let title = title.to_string();
    command::register(meta, move || last_action.set(title.clone()));
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    // Load Lua plugins BEFORE building the UI, so their commands, shortcuts and
    // status items exist by the time the tree is assembled.
    match baseui_lua::LuaEngine::new() {
        Ok(engine) => {
            let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/plugins");
            let n = engine.load_dir(dir);
            log::info!("loaded {n} lua plugin(s)");
            // Keep the engine alive for the process; its commands/handlers hold
            // Lua functions.
            std::mem::forget(engine);
        }
        Err(e) => log::error!("lua init failed: {e}"),
    }

    let selected = baseui::core::create_signal(String::from("Cube"));
    let last_action = baseui::core::create_signal(String::from("Ready"));
    let grid_on = baseui::core::create_signal(true);
    let snap_on = baseui::core::create_signal(false);

    // Event bus: react to selection changes without the tree and the
    // viewport/status bar referencing each other directly.
    bus::subscribe::<SelectionChanged>(move |e| {
        selected.set(e.name.clone());
        last_action.set(format!("Selected {}", e.name));
    })
    .leak();

    let blue = col(0x4d, 0x9c, 0xf5);
    let orange = col(0xe0, 0x8a, 0x3c);
    let green = col(0x6c, 0xc6, 0x8a);
    let purple = col(0xc7, 0x6c, 0xd6);

    // --- Register commands (menus, toolbar, shortcuts, and the palette all
    //     invoke these by id) --------------------------------------------------
    cmd(last_action, "file.new", "New", "File", gis::MAP, blue, Some("Ctrl+N"));
    cmd(last_action, "file.open", "Open File…", "File", gis::FOLDER_MAP, blue, Some("Ctrl+O"));
    cmd(last_action, "file.save", "Save", "File", gis::MAP_SEND, blue, Some("Ctrl+S"));
    cmd(last_action, "file.quit", "Quit", "File", gis::MAP_RM, blue, None);
    cmd(last_action, "edit.undo", "Undo", "Edit", gis::COMPASS, purple, Some("Ctrl+Z"));
    cmd(last_action, "edit.redo", "Redo", "Edit", gis::COMPASS_ALT, purple, Some("Ctrl+Shift+Z"));
    cmd(last_action, "create.cube", "Create Cube", "Create", gis::POLYGON, orange, None);
    cmd(last_action, "create.cube.opts", "Cube Options…", "Create", glyphs::GEAR, orange, None);
    cmd(last_action, "create.sphere", "Create Sphere", "Create", gis::GLOBE, orange, None);
    cmd(last_action, "create.sphere.opts", "Sphere Options…", "Create", glyphs::GEAR, orange, None);
    cmd(last_action, "create.plane", "Create Plane", "Create", gis::LAYERS, orange, None);
    cmd(last_action, "create.plane.opts", "Plane Options…", "Create", glyphs::GEAR, orange, None);
    // Global text scale — commands, so they work from the menu, a shortcut, and
    // the Command Palette alike.
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

    cmd(last_action, "view.grid", "Toggle Grid", "View", gis::LAYER, green, Some("G"));
    cmd(last_action, "view.measure", "Measure Tool", "View", gis::MEASURE, green, Some("M"));
    cmd(last_action, "view.point", "Point Tool", "View", gis::POINT, green, Some("P"));

    // --- Menu bar ---------------------------------------------------------
    let menubar = MenuBar::new()
        .menu(
            Menu::new("File")
                .item_icon(gis::MAP, "New", || command::run("file.new"))
                .item_icon(gis::FOLDER_MAP, "Open File…", || command::run("file.open"))
                .item_icon(gis::MAP_SEND, "Save", || command::run("file.save"))
                .separator()
                .item("Quit", || command::run("file.quit")),
        )
        .menu(
            Menu::new("Create")
                .item_options(gis::POLYGON, "Cube", || command::run("create.cube"), || command::run("create.cube.opts"))
                .item_options(gis::GLOBE, "Sphere", || command::run("create.sphere"), || command::run("create.sphere.opts"))
                .item_options(gis::LAYERS, "Plane", || command::run("create.plane"), || command::run("create.plane.opts")),
        )
        .menu(
            Menu::new("Edit")
                .item("Undo", || command::run("edit.undo"))
                .item("Redo", || command::run("edit.redo")),
        )
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
        .menu(
            Menu::new("Window")
                .item_icon(glyphs::SQUARE, "Open Tool Window", || {
                    command::run("window.tool")
                }),
        )
        .menu(Menu::new("Help").item_icon(glyphs::STAR, "Command Palette  (F1)", || {}));

    // --- Toolbar ----------------------------------------------------------
    let toolbar = Toolbar::new()
        .button_icon(gis::MAP, || command::run("file.new"))
        .button_icon(gis::MAP_SEND, || command::run("file.save"))
        .separator()
        .button_icon(gis::POINT, || command::run("view.point"))
        .button_icon(gis::POLYGON, || command::run("create.cube"))
        .button_icon(gis::MEASURE, || command::run("view.measure"))
        .separator()
        .toggle_icon(gis::LAYER, grid_on)
        .toggle_icon(gis::MOVE, snap_on)
        .spacer()
        .button_labeled(gis::MAP_OPTIONS, "Options", || command::run("view.grid"));

    // --- Outliner (gis type icons + toggle icons) -------------------------
    let eye_on = col(0xd8, 0xd8, 0xde);
    let obj = |name: &str, icon: Icon, color: Color, vis: bool, rend: bool| {
        TreeNode::leaf(name)
            .icon(icon)
            .icon_color(color)
            .action(glyphs::EYE, eye_on, vis)
            .action(glyphs::DIAMOND, blue, rend)
    };
    let tree = TreeView::new(vec![TreeNode::branch(
        "Scene Collection",
        vec![TreeNode::branch(
            "Collection",
            vec![
                obj("Camera", gis::COMPASS, green, true, true),
                obj("Cube", gis::POLYGON, orange, true, true),
                obj("Light", glyphs::STAR, col(0xe6, 0xc2, 0x4e), true, false),
                obj("Sphere", gis::GLOBE, orange, false, true),
            ],
        )
        .icon(gis::LAYERS)
        .icon_color(green)],
    )
    .icon(gis::GLOBE)
    .icon_color(blue)])
    .on_select(|label| {
        // Typed event for Rust subscribers...
        bus::publish(&SelectionChanged { name: label.to_string() });
        // ...and the named channel, which scripts subscribe to.
        bus::publish_named("selection.changed", serde_json::json!({ "name": label }));
    })
    // Right-click a row for a context menu (the same PopupMenu the menu bar and
    // combo boxes use). Dock tabs will use this too.
    .context_menu(
        vec![
            MenuItemSpec::new("Rename").icon(glyphs::GEAR),
            MenuItemSpec::new("Duplicate").shortcut("Ctrl+D"),
            MenuItemSpec::separator(),
            MenuItemSpec::new("Detach to Window").icon(glyphs::SQUARE),
            MenuItemSpec::separator(),
            MenuItemSpec::new("Delete").disabled(),
        ],
        move |node, item| {
            let action = match item {
                0 => "Rename",
                1 => "Duplicate",
                3 => "Detach to Window",
                _ => "?",
            };
            if item == 3 {
                command::run("window.tool");
            }
            last_action.set(format!("{action}: {node}"));
        },
    )
    .persist("tree.outliner");

    // --- Tabbed inspector -------------------------------------------------
    let location = [sig(0.0), sig(0.0), sig(0.0)];
    let rotation = [sig(0.0), sig(0.0), sig(0.0)];
    let scale = [sig(1.0), sig(1.0), sig(1.0)];
    let fov = sig(50.0);
    let name = baseui::core::create_signal(String::from("Cube"));
    let mode = baseui::core::create_signal(0usize);
    let passcode = baseui::core::create_signal(String::new());
    let object_tab = ScrollArea::new(
        PropertyView::new()
            .group(
                PropGroup::new("Object")
                    .icon_color(blue)
                    .row("Name", TextBox::new(name).placeholder("Object name"))
                    .row(
                        "Mode",
                        ComboBox::new(
                            mode,
                            [
                                "XYZ Euler",
                                "XZY Euler",
                                "YXZ Euler",
                                "Quaternion (WXYZ)",
                                "Axis Angle",
                            ],
                        ),
                    )
                    .row("Passcode", TextBox::new(passcode).password().placeholder("••••")),
            )
            .group(xyz("Location", orange, location, 0.01))
            .group(xyz("Rotation", purple, rotation, 0.5))
            .group(xyz("Scale", green, scale, 0.01))
            .group(
                PropGroup::new("Camera")
                    .icon_color(green)
                    .row("FOV", DragValue::new(fov).range(1.0, 179.0).speed(0.25).decimals(1)),
            )
            .persist("props.object"),
    );
    let (mr, mg, mb, rough) = (sig(0.8), sig(0.3), sig(0.2), sig(0.4));

    // A floating tool window. Its sliders are bound to the SAME signals as the
    // main window's Material tab, so dragging one repaints the other: windows
    // share the GPU device, the theme, and the reactive runtime.
    command::register(
        CommandMeta::new("window.tool", "Open Tool Window")
            .category("Window")
            .icon(glyphs::SQUARE)
            .color(blue)
            .shortcut("Ctrl+T"),
        move || {
            window::open(
                WindowSpec::new(
                    "BaseUI — Tool Window",
                    Column::new()
                        .padding(baseui::Insets::all(14.0))
                        .spacing(10.0)
                        .child(Label::new("Detached tool window").size(17.0))
                        .child(
                            Label::new("Shares signals with the main window:")
                                .color(col(0x9a, 0x9a, 0xa4)),
                        )
                        .child(
                            Label::dynamic(move || {
                                format!(
                                    "R {:.2}   G {:.2}   B {:.2}",
                                    mr.get(),
                                    mg.get(),
                                    mb.get()
                                )
                            })
                            .size(16.0)
                            .color(orange),
                        )
                        .child(Slider::new(mr).range(0.0, 1.0).width(260.0))
                        .child(Slider::new(mg).range(0.0, 1.0).width(260.0))
                        .child(Slider::new(mb).range(0.0, 1.0).width(260.0)),
                )
                .size(340, 260),
            );
        },
    );
    let material_tab = ScrollArea::new(
        PropertyView::new().group(
            PropGroup::new("Surface")
                .icon_color(orange)
                .row("Base R", Slider::new(mr).range(0.0, 1.0).width(180.0))
                .row("Base G", Slider::new(mg).range(0.0, 1.0).width(180.0))
                .row("Base B", Slider::new(mb).range(0.0, 1.0).width(180.0))
                .row("Roughness", Slider::new(rough).range(0.0, 1.0).width(180.0)),
        ),
    );
    // Extra panes so the vertical rail has something to switch between.
    let (subdiv, bevel) = (sig(2.0), sig(0.1));
    let modifiers_tab = ScrollArea::new(
        PropertyView::new()
            .group(
                PropGroup::new("Subdivision")
                    .icon_color(blue)
                    .row("Levels", Slider::new(subdiv).range(0.0, 6.0).width(180.0))
                    .row("Render", DragValue::new(subdiv).range(0.0, 6.0).speed(0.05).decimals(0)),
            )
            .group(
                PropGroup::new("Bevel")
                    .icon_color(purple)
                    .row("Amount", DragValue::new(bevel).range(0.0, 1.0).speed(0.005).decimals(3)),
            ),
    );
    let (gravity, samples) = (sig(9.81), sig(64.0));
    let world_tab = ScrollArea::new(
        PropertyView::new().group(
            PropGroup::new("World")
                .icon_color(green)
                .row("Gravity", DragValue::new(gravity).range(0.0, 30.0).speed(0.05).decimals(2))
                .row("Strength", Slider::new(sig(1.0)).range(0.0, 4.0).width(180.0)),
        ),
    );
    let render_tab = ScrollArea::new(
        PropertyView::new().group(
            PropGroup::new("Sampling")
                .icon_color(orange)
                .row("Samples", DragValue::new(samples).range(1.0, 4096.0).speed(2.0).decimals(0))
                .row("Denoise", baseui::widget::Checkbox::new(
                    baseui::core::create_signal(true),
                    "Enabled",
                )),
        ),
    );

    // Blender-style: a vertical icon rail on the left picks which pane shows.
    let inspector = TabView::new()
        .vertical()
        .tab_icon(gis::POLYGON, "Object", object_tab)
        .tab_icon(glyphs::GEAR, "Modifiers", modifiers_tab)
        .tab_icon(glyphs::CIRCLE, "Material", material_tab)
        .tab_icon(gis::GLOBE, "World", world_tab)
        .tab_icon(gis::MAP, "Render", render_tab)
        .persist("tabs.inspector");

    // --- Outliner pane: search box above the scrolling tree ---------------
    let search = baseui::core::create_signal(String::new());
    let tree_pane = Split::vertical()
        .gutter(0.0)
        .fixed_range(
            38.0,
            38.0,
            38.0,
            Column::new()
                .padding(baseui::Insets::symmetric(6.0, 5.0))
                .child(TextBox::new(search).placeholder("Search…")),
        )
        .flex(ScrollArea::new(tree).persist("scroll.tree"));

    // --- Center split -----------------------------------------------------
    let center = Split::horizontal()
        .fixed_range(260.0, 160.0, 420.0, tree_pane)
        .flex(Viewport { selected })
        .fixed_range(360.0, 260.0, 560.0, inspector)
        .persist("split.center");

    // --- Status bar -------------------------------------------------------
    let status = StatusBar::new()
        .item(StatusItem::dynamic(move || last_action.get()).icon(glyphs::CHECK).color(green))
        .item(StatusItem::dynamic(move || format!("Selection: {}", selected.get())))
        .item(StatusItem::new("Press F1 for commands").right().color(blue))
        .item(StatusItem::dynamic(|| {
            format!("Text {:.0}%", baseui::text::scale() * 100.0)
        })
        .right())
        .item(StatusItem::new("BaseUI M7").right());

    // --- Frame ------------------------------------------------------------
    let root = Split::vertical()
        .gutter(0.0)
        .fixed_range(30.0, 30.0, 30.0, menubar)
        .fixed_range(40.0, 40.0, 40.0, toolbar)
        .flex(center)
        .fixed_range(26.0, 26.0, 26.0, status);

    // Scripted-demo hook: queue the tool window before the loop starts. The
    // request is drained once the event loop goes idle, after the main window
    // (and the GPU context) exist.
    if std::env::var_os("BASEUI_OPEN_TOOL").is_some() {
        command::run("window.tool");
    }

    App::new()
        .with_title("BaseUI — App Frame")
        .with_size(1200, 780)
        .with_persistence(std::env::temp_dir().join("baseui-inspector-state.json"))
        .with_root(root)
        .run()
}
