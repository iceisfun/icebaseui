//! BaseUI code-editor demo.
//!
//! A [`TextArea`] configured as a code area:
//!
//! - **line numbers**, current-line highlight, click/drag selection,
//! - a **highlighter** — a closure that colours each visible line's tokens,
//! - a **checker** — a closure that returns diagnostics, re-run on every edit and
//!   drawn as squiggly underlines,
//! - and a second, proportional TextArea for plain notes: same widget, no
//!   monospace, no gutter.
//!
//! Both closures here are deliberately dumb: BaseUI does not ship a parser, it
//! ships the *seam*. Point `highlighter` at a real lexer and `checker` at a real
//! language server and you have a real editor.
//!
//! ```text
//! cargo run -p editor
//! ```

use baseui::command::{self, CommandMeta};
use baseui::icon::{gis, glyphs};
use baseui::widget::{
    Diagnostic, DockArea, DockAxis, DockNode, Menu, MenuBar, Panel, Span, Split, StatusBar,
    StatusItem, TextArea,
};
use baseui::{App, Color};

const KEYWORDS: &[&str] = &[
    "fn", "let", "mut", "pub", "struct", "enum", "impl", "for", "in", "if", "else", "match",
    "return", "use", "mod", "self", "Self", "true", "false", "while", "loop", "const", "type",
    "where", "as", "dyn", "move", "ref",
];

fn col(r: u8, g: u8, b: u8) -> Color {
    Color::rgb8(r, g, b)
}

/// Colour one line of Rust-ish source.
///
/// Walks the line once, emitting a [`Span`] per token in **character** columns
/// (which is what the TextArea indexes by). Comments and strings are checked
/// first so they win over everything inside them.
fn highlight(line: &str) -> Vec<Span> {
    let keyword = col(0xc5, 0x92, 0xe8);
    let string = col(0x8f, 0xc9, 0x7a);
    let number = col(0xe0, 0xa0, 0x60);
    let comment = col(0x70, 0x76, 0x84);
    let function = col(0x62, 0xa8, 0xf0);
    let macro_ = col(0xe6, 0xc2, 0x4e);

    let chars: Vec<char> = line.chars().collect();
    let mut spans = Vec::new();
    let mut i = 0;

    while i < chars.len() {
        // Line comment: colour the rest and stop.
        if chars[i] == '/' && chars.get(i + 1) == Some(&'/') {
            spans.push(Span {
                start: i,
                end: chars.len(),
                color: comment,
            });
            break;
        }

        // String literal.
        if chars[i] == '"' {
            let start = i;
            i += 1;
            while i < chars.len() {
                match chars[i] {
                    '\\' => i += 2,
                    '"' => {
                        i += 1;
                        break;
                    }
                    _ => i += 1,
                }
            }
            spans.push(Span {
                start,
                end: i.min(chars.len()),
                color: string,
            });
            continue;
        }

        // Number.
        if chars[i].is_ascii_digit() {
            let start = i;
            while i < chars.len() && (chars[i].is_alphanumeric() || chars[i] == '.') {
                i += 1;
            }
            spans.push(Span {
                start,
                end: i,
                color: number,
            });
            continue;
        }

        // Identifier — keyword, macro (`name!`), call (`name(`), or plain.
        if chars[i].is_alphabetic() || chars[i] == '_' {
            let start = i;
            while i < chars.len() && (chars[i].is_alphanumeric() || chars[i] == '_') {
                i += 1;
            }
            let word: String = chars[start..i].iter().collect();
            let color = if KEYWORDS.contains(&word.as_str()) {
                Some(keyword)
            } else if chars.get(i) == Some(&'!') {
                Some(macro_)
            } else if chars.get(i) == Some(&'(') {
                Some(function)
            } else {
                None
            };
            if let Some(color) = color {
                spans.push(Span {
                    start,
                    end: i,
                    color,
                });
            }
            continue;
        }

        i += 1;
    }

    spans
}

/// A stand-in "compiler": flags every `unwrap()` as an error and every `TODO` as
/// a warning. Re-run by the TextArea after each edit, so the squiggles track the
/// text as you type.
fn check(text: &str) -> Vec<Diagnostic> {
    let error = col(0xe0, 0x5a, 0x5a);
    let warning = col(0xe0, 0xa8, 0x40);
    let mut out = Vec::new();

    for (n, line) in text.lines().enumerate() {
        let chars: Vec<char> = line.chars().collect();
        for (needle, color) in [("unwrap()", error), ("TODO", warning)] {
            let needle: Vec<char> = needle.chars().collect();
            if chars.len() < needle.len() {
                continue;
            }
            for start in 0..=chars.len() - needle.len() {
                if chars[start..start + needle.len()] == needle[..] {
                    out.push(Diagnostic {
                        line: n,
                        start,
                        end: start + needle.len(),
                        color,
                    });
                }
            }
        }
    }
    out
}

const SAMPLE: &str = r#"//! A BaseUI widget: three passes, no framework magic.

use baseui::widget::{EventCx, LayoutCx, PaintCx, Widget};

pub struct Counter {
    count: i32,
    label: String,
}

impl Widget for Counter {
    fn layout(&mut self, cx: &mut LayoutCx, c: Constraints) -> Size {
        let text = format!("{}: {}", self.label, self.count);
        let size = cx.fonts.measure(&text, 14.0, FontId::Ui);
        c.constrain(size + Size::new(24.0, 12.0))
    }

    fn paint(&mut self, cx: &mut PaintCx, bounds: Rect, scene: &mut Scene) {
        // TODO: hover and pressed states
        scene.rounded_rect(bounds, cx.theme.palette.surface, 6.0);
        scene.text(bounds.origin(), self.render().unwrap(), 14.0, WHITE);
    }

    fn event(&mut self, cx: &mut EventCx, bounds: Rect, event: &InputEvent) {
        if let InputEvent::PointerPressed { pos, .. } = event {
            if bounds.contains(*pos) {
                self.count += 1;
                cx.consume(); // children before chrome
            }
        }
    }
}
"#;

const NOTES: &str = "\
Notes

- click and drag to select; shift+arrows to extend
- ctrl+A / ctrl+C / ctrl+X / ctrl+V
- ctrl+Z undoes, ctrl+shift+Z (or ctrl+Y) redoes
  a run of typing undoes as one word, not one letter
- ctrl+Home / ctrl+End jump to the ends of the document
- ctrl+left / ctrl+right jump by word
- the gutter tracks the caret line
- unwrap() and TODO get squiggled in the code pane; type one in

This pane is the same TextArea, only proportional and without a gutter.
Lines do not wrap — that is what keeps the caret a prefix sum.";

fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    let blue = col(0x4d, 0x9c, 0xf5);
    let green = col(0x6c, 0xc6, 0x8a);

    let code = TextArea::new(SAMPLE)
        .line_numbers()
        .font_size(13.0)
        .highlighter(highlight)
        .checker(check)
        .undo_history();

    let notes = TextArea::new(NOTES)
        .proportional()
        .font_size(14.0)
        .undo_history();

    let dock = DockArea::new(DockNode::split(
        DockAxis::Horizontal,
        vec![DockNode::tabs(["code"]), DockNode::tabs(["notes"])],
    ))
    .panel(Panel::new("code", "counter.rs", code).icon(gis::SHAPE_FILE))
    .panel(Panel::new("notes", "Notes", notes).icon(gis::MAP_EDIT))
    .persist("editor.layout");

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

    let menubar = MenuBar::new()
        .menu(Menu::new("File").item("Quit", || {}))
        .menu(
            Menu::new("View")
                .item_icon(glyphs::CIRCLE, "Increase Text Size", || {
                    command::run("view.text.inc")
                })
                .item_icon(glyphs::CIRCLE_OUTLINE, "Decrease Text Size", || {
                    command::run("view.text.dec")
                }),
        );

    let status = StatusBar::new()
        .item(
            StatusItem::new("Type in the code pane — diagnostics update live")
                .icon(glyphs::CHECK)
                .color(green),
        )
        .item(StatusItem::new("F1 for commands").right().color(blue));

    let root = Split::vertical()
        .gutter(0.0)
        .fixed_range(30.0, 30.0, 30.0, menubar)
        .flex(dock)
        .fixed_range(26.0, 26.0, 26.0, status);

    App::new()
        .with_title("BaseUI — Editor")
        .with_size(1100, 720)
        .with_persistence(std::env::temp_dir().join("baseui-editor-state.json"))
        .with_root(root)
        .run()
}
