//! The BaseUI widget + reactivity demo (milestone M3).
//!
//! A retained widget tree bound to a reactive signal: the count [`Label`] reads
//! the signal, the buttons write it, and the reactive change hook repaints
//! automatically. Exercises layout (Column/Row), input routing, and the
//! retained + reactive model end to end.
//!
//! ```text
//! cargo run -p counter
//! ```

use baseui::core::create_signal;
use baseui::widget::{Button, Column, Label, Row};
use baseui::{App, Insets, Theme};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    // The single piece of application state. `Signal` is `Copy`, so we can hand
    // a copy to each closure below.
    let count = create_signal(0i32);

    let title = Label::new("BaseUI — Reactive Counter").size(20.0);

    let value = Label::dynamic(move || format!("Count: {}", count.get()))
        .size(34.0)
        .color(Theme::dark().palette.accent);

    let buttons = Row::new()
        .spacing(8.0)
        .child(Button::new("–").on_click(move || count.update(|c| *c -= 1)))
        .child(Button::new("Reset").on_click(move || count.set(0)))
        .child(
            Button::new("+")
                .primary()
                .on_click(move || count.update(|c| *c += 1)),
        );

    let root = Column::new()
        .padding(Insets::all(28.0))
        .spacing(16.0)
        .child(title)
        .child(value)
        .child(buttons);

    App::new()
        .with_title("BaseUI — Counter")
        .with_size(520, 320)
        .with_root(root)
        .run()
}
