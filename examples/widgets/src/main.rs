//! BaseUI core-widget gallery (milestone M4), starring [`HexView`].
//!
//! Everything here is bound to reactive signals: the checkbox live-toggles the
//! HexView's ASCII pane, the Blender-style drag-value and the slider feed labels
//! that update as you scrub. Scroll the wheel over the hex dump to page through
//! the buffer; hover a byte to highlight it in both panes.
//!
//! ```text
//! cargo run -p widgets
//! ```

use baseui::core::create_signal;
use baseui::widget::{Checkbox, Column, DragValue, HexView, Label, Row, Slider};
use baseui::{App, Insets, Theme};

fn sample_data() -> Vec<u8> {
    let mut data = Vec::new();
    data.extend_from_slice(b"BaseUI HexView\x00\x01\x02 demo buffer\n\t");
    // A full 0x00..=0xFF ramp exercises every byte class / color.
    data.extend(0u8..=255u8);
    data.extend_from_slice(b"The quick brown fox jumps over the lazy dog. 0123456789!?");
    data.extend(std::iter::repeat_n(0u8, 24));
    data.extend_from_slice(&[0xDE, 0xAD, 0xBE, 0xEF, 0xCA, 0xFE, 0xBA, 0xBE]);
    data
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    let show_ascii = create_signal(true);
    let fov = create_signal(60.0f32);
    let zoom = create_signal(1.0f32);
    let accent = Theme::dark().palette.accent;

    let controls = Column::new()
        .spacing(14.0)
        .child(Label::new("Controls").size(18.0))
        .child(Checkbox::new(show_ascii, "Show ASCII pane"))
        .child(Label::new("Camera FOV (drag to scrub)"))
        .child(
            DragValue::new(fov)
                .label("FOV")
                .range(1.0, 179.0)
                .speed(0.4)
                .decimals(1),
        )
        .child(Label::dynamic(move || format!("→ {:.1}°", fov.get())).color(accent))
        .child(Label::new("Zoom"))
        .child(Slider::new(zoom).range(0.25, 4.0).width(220.0))
        .child(Label::dynamic(move || format!("→ {:.2}×", zoom.get())).color(accent));

    let hex = Column::new()
        .spacing(10.0)
        .child(Label::new("HexView — colored bytes, hover + wheel-scroll").size(18.0))
        .child(
            HexView::new(sample_data())
                .rows(22)
                .font_size(13.0)
                .ascii_toggle(show_ascii),
        );

    let root = Row::new()
        .padding(Insets::all(24.0))
        .spacing(36.0)
        .child(controls)
        .child(hex);

    App::new()
        .with_title("BaseUI — Widget Gallery")
        .with_size(1120, 760)
        .with_root(root)
        .run()
}
