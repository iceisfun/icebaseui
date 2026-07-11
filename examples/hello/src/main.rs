//! The BaseUI foundation demo: open a themed window.
//!
//! For the foundation milestone this opens a single window and clears it to the
//! active theme's background color. Run with:
//!
//! ```text
//! cargo run -p hello
//! ```
//!
//! Set `BASEUI_THEME=light` to preview the light theme, and `RUST_LOG=info` for
//! backend logging.

use baseui::{App, Theme};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    let theme = match std::env::var("BASEUI_THEME").as_deref() {
        Ok("light") => Theme::light(),
        _ => Theme::dark(),
    };

    App::new()
        .with_title("BaseUI — Hello")
        .with_size(1000, 700)
        .with_theme(theme)
        .run()
}
