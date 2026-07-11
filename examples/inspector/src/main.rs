//! BaseUI flagship demo (milestone M5): an Outliner-style [`TreeView`] beside a
//! Properties-style [`PropertyView`], echoing the Blender reference.
//!
//! - The tree has expand/collapse arrows, colored type icons, hover, and
//!   single selection (click a row).
//! - The property inspector has collapsible groups with colored section icons;
//!   each row's editor is a real [`DragValue`]/[`Slider`]/[`Checkbox`] bound to a
//!   signal — drag the Location/Rotation/Scale fields to scrub them.
//! - Both panels scroll (mouse wheel).
//!
//! ```text
//! cargo run -p inspector
//! ```

use baseui::core::create_signal;
use baseui::widget::{
    Checkbox, Column, DragValue, Label, PropGroup, PropertyView, Row, ScrollArea, Slider, TreeNode,
    TreeView,
};
use baseui::{App, Color, Insets, Signal};

fn color(r: u8, g: u8, b: u8) -> Color {
    Color::rgb8(r, g, b)
}

fn transform_group(title: &str, icon: Color, xyz: [Signal<f32>; 3], speed: f32) -> PropGroup {
    let [x, y, z] = xyz;
    PropGroup::new(title.to_string())
        .icon_color(icon)
        .row("X", DragValue::new(x).speed(speed).decimals(3))
        .row("Y", DragValue::new(y).speed(speed).decimals(3))
        .row("Z", DragValue::new(z).speed(speed).decimals(3))
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    // --- Outliner ---------------------------------------------------------
    let camera = color(0x6c, 0xc6, 0x8a);
    let mesh = color(0xe0, 0x8a, 0x3c);
    let light = color(0xe6, 0xc2, 0x4e);
    let mat = color(0x9a, 0x9a, 0xa4);

    let tree = TreeView::new(vec![TreeNode::branch(
        "Scene Collection",
        vec![
            TreeNode::branch(
                "Collection",
                vec![
                    TreeNode::leaf("Camera").icon_color(camera),
                    TreeNode::leaf("Cube").icon_color(mesh),
                    TreeNode::leaf("Light").icon_color(light),
                ],
            ),
            TreeNode::branch(
                "Materials",
                vec![
                    TreeNode::leaf("Metal").icon_color(mat),
                    TreeNode::leaf("Glass").icon_color(color(0x62, 0xb6, 0xd6)),
                ],
            ),
            TreeNode::branch(
                "Modifiers",
                vec![
                    TreeNode::leaf("Subdivision"),
                    TreeNode::leaf("Bevel"),
                    TreeNode::leaf("Mirror"),
                ],
            )
            .collapsed(),
        ],
    )])
    .on_select(|label| eprintln!("selected: {label}"));

    let outliner = Column::new()
        .spacing(8.0)
        .child(Label::new("Outliner").size(15.0))
        .child(ScrollArea::new(tree).width(280.0).height(660.0));

    // --- Properties -------------------------------------------------------
    let location = [create_signal(0.0), create_signal(0.0), create_signal(0.0)];
    let rotation = [create_signal(0.0), create_signal(0.0), create_signal(0.0)];
    let scale = [create_signal(1.0), create_signal(1.0), create_signal(1.0)];
    let fov = create_signal(50.0f32);
    let near = create_signal(0.1f32);
    let far = create_signal(1000.0f32);
    let shadows = create_signal(true);
    let ao = create_signal(0.8f32);

    let props = PropertyView::new()
        .group(transform_group("Location", mesh, location, 0.01))
        .group(transform_group("Rotation", color(0x8a, 0x8a, 0xf0), rotation, 0.5))
        .group(transform_group("Scale", color(0x6c, 0xc6, 0x8a), scale, 0.01))
        .group(
            PropGroup::new("Camera")
                .icon_color(camera)
                .row("FOV", DragValue::new(fov).range(1.0, 179.0).speed(0.25).decimals(1))
                .row("Near", DragValue::new(near).range(0.001, 10.0).speed(0.01).decimals(3))
                .row("Far", DragValue::new(far).range(1.0, 100000.0).speed(5.0).decimals(0)),
        )
        .group(
            PropGroup::new("Rendering")
                .icon_color(color(0xc7, 0x6c, 0xd6))
                .row("Shadows", Checkbox::new(shadows, "Enabled"))
                .row("Ambient", Slider::new(ao).range(0.0, 1.0).width(200.0)),
        );

    let inspector = Column::new()
        .spacing(8.0)
        .child(Label::new("Properties").size(15.0))
        .child(ScrollArea::new(props).width(400.0).height(660.0));

    let root = Row::new()
        .padding(Insets::all(20.0))
        .spacing(28.0)
        .child(outliner)
        .child(inspector);

    App::new()
        .with_title("BaseUI — Inspector")
        .with_size(820, 760)
        .with_root(root)
        .run()
}
