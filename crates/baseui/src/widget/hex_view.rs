//! [`HexView`] — a monospace hex/ASCII dump viewer with per-byte coloring,
//! hover highlighting, and wheel scrolling.
//!
//! This is the first widget to lean on the framework's monospace font and text
//! measurement (from M3): every column is placed on an exact character grid via
//! [`Fonts::char_advance`](crate::text::Fonts::char_advance), and each byte is
//! drawn as its own colored run — the "colored HH / ASCII parts" use case from
//! `docs/rich-text.md`, buildable today without the future styled-run engine.
//!
//! Layout (16 bytes/row):
//!
//! ```text
//! 00000000  48 65 6C 6C 6F 20 77 6F  72 6C 64 00 01 02 7F 80  Hello world.....
//! └offset┘  └────── hex bytes, grouped 8+8 ──────┘           └──── ascii ────┘
//! ```

use baseui_core::Signal;
use baseui_core::paint::{Scene, TextShape};
use baseui_core::{Color, Insets, Point, Rect, Size};

use super::{EventCx, LayoutCx, PaintCx, Widget};
use crate::event::InputEvent;
use crate::layout::Constraints;
use crate::text::FontId;
use crate::theme::Palette;

const BYTES_PER_ROW: usize = 16;
const OFFSET_CHARS: usize = 8;
/// First hex byte begins this many characters from the left.
const HEX_START: usize = OFFSET_CHARS + 2;
/// First ASCII character column (after the hex area + its group gap + a gutter).
const ASCII_START: usize = HEX_START + BYTES_PER_ROW * 3 + 1 + 2;
const TOTAL_CHARS: usize = ASCII_START + BYTES_PER_ROW;
/// Chars up to and including the hex area (used when the ASCII pane is hidden).
const HEX_TOTAL_CHARS: usize = HEX_START + BYTES_PER_ROW * 3 + 1;

/// Coarse classification of a byte, driving its color.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum ByteClass {
    Zero,
    Printable,
    Control,
    High,
}

fn classify(b: u8) -> ByteClass {
    match b {
        0 => ByteClass::Zero,
        0x20..=0x7e => ByteClass::Printable,
        0x80.. => ByteClass::High,
        _ => ByteClass::Control,
    }
}

fn class_color(class: ByteClass, p: &Palette) -> Color {
    match class {
        ByteClass::Zero => p.text_muted,
        ByteClass::Printable => p.text,
        ByteClass::Control => p.warning,
        ByteClass::High => p.accent,
    }
}

/// Character-grid x (in char units) of hex byte column `c`, accounting for the
/// extra space between the two 8-byte groups.
fn hex_col_char(c: usize) -> usize {
    HEX_START + c * 3 + if c >= BYTES_PER_ROW / 2 { 1 } else { 0 }
}

/// A hex/ASCII dump viewer over an owned byte buffer.
pub struct HexView {
    data: Vec<u8>,
    rows: usize,
    font_size: f32,
    top_row: usize,
    hovered: Option<usize>,
    ascii_visible: Option<Signal<bool>>,
}

impl HexView {
    pub fn new(data: impl Into<Vec<u8>>) -> Self {
        HexView {
            data: data.into(),
            rows: 16,
            font_size: 13.0,
            top_row: 0,
            hovered: None,
            ascii_visible: None,
        }
    }

    /// Number of rows shown in the viewport (the view scrolls within the data).
    pub fn rows(mut self, rows: usize) -> Self {
        self.rows = rows.max(1);
        self
    }

    /// Monospace font size in logical pixels.
    pub fn font_size(mut self, size: f32) -> Self {
        self.font_size = size;
        self
    }

    /// Bind ASCII-pane visibility to a signal (e.g. a checkbox), so toggling it
    /// re-lays-out the view live.
    pub fn ascii_toggle(mut self, visible: Signal<bool>) -> Self {
        self.ascii_visible = Some(visible);
        self
    }

    fn ascii_on(&self) -> bool {
        self.ascii_visible.map(|s| s.get()).unwrap_or(true)
    }

    fn total_rows(&self) -> usize {
        self.data.len().div_ceil(BYTES_PER_ROW)
    }

    fn max_top(&self) -> usize {
        self.total_rows().saturating_sub(self.rows)
    }

    /// Map a pointer position (absolute) to the byte index under it, if any.
    fn byte_at(&self, cx_fonts: &crate::text::Fonts, bounds: Rect, pos: Point, pad: f32) -> Option<usize> {
        let char_w = cx_fonts.char_advance('0', self.font_size, FontId::Mono);
        let row_h = cx_fonts.line_height(self.font_size, FontId::Mono);
        let inner_left = bounds.left() + pad;
        let inner_top = bounds.top() + pad;

        if pos.y < inner_top || pos.x < inner_left {
            return None;
        }
        let r = ((pos.y - inner_top) / row_h) as usize;
        if r >= self.rows {
            return None;
        }
        let relx = (pos.x - inner_left) / char_w;

        // Hex cells.
        for c in 0..BYTES_PER_ROW {
            let start = hex_col_char(c) as f32;
            if relx >= start && relx < start + 2.0 {
                return self.index(r, c);
            }
        }
        // ASCII cells.
        if self.ascii_on() {
            for c in 0..BYTES_PER_ROW {
                let start = (ASCII_START + c) as f32;
                if relx >= start && relx < start + 1.0 {
                    return self.index(r, c);
                }
            }
        }
        None
    }

    fn index(&self, visible_row: usize, col: usize) -> Option<usize> {
        let idx = (self.top_row + visible_row) * BYTES_PER_ROW + col;
        (idx < self.data.len()).then_some(idx)
    }
}

impl Widget for HexView {
    fn layout(&mut self, cx: &mut LayoutCx<'_>, constraints: Constraints) -> Size {
        let char_w = cx.fonts.char_advance('0', self.font_size, FontId::Mono);
        let row_h = cx.fonts.line_height(self.font_size, FontId::Mono);
        let pad = cx.theme.spacing.md;

        let cols = if self.ascii_on() {
            TOTAL_CHARS
        } else {
            HEX_TOTAL_CHARS
        };
        let size = Size::new(
            cols as f32 * char_w + pad * 2.0,
            self.rows as f32 * row_h + pad * 2.0,
        );
        constraints.constrain(size)
    }

    fn paint(&mut self, cx: &mut PaintCx<'_>, bounds: Rect, scene: &mut Scene) {
        let p = &cx.theme.palette;
        let pad = cx.theme.spacing.md;
        let char_w = cx.fonts.char_advance('0', self.font_size, FontId::Mono);
        let row_h = cx.fonts.line_height(self.font_size, FontId::Mono);
        let ascii = self.ascii_on();

        // Panel background.
        scene.push_rect(
            baseui_core::paint::RectShape::fill(bounds, p.surface)
                .with_corner_radius(cx.theme.radius.md)
                .with_border(1.0, p.border),
        );
        scene.push_clip(bounds.shrink(Insets::all(1.0)));

        let inner_left = bounds.left() + pad;
        let inner_top = bounds.top() + pad;

        let mono = |scene: &mut Scene, ch: usize, row: f32, text: String, color: Color| {
            scene.push_text(TextShape {
                pos: Point::new(inner_left + ch as f32 * char_w, row),
                text,
                size: self.font_size,
                color,
                mono: true,
            });
        };

        let top = self.top_row.min(self.max_top());
        for r in 0..self.rows {
            let row_index = top + r;
            let base = row_index * BYTES_PER_ROW;
            if base >= self.data.len() {
                break;
            }
            let y = inner_top + r as f32 * row_h;

            // Offset column.
            mono(scene, 0, y, format!("{base:08X}"), p.text_muted);

            for c in 0..BYTES_PER_ROW {
                let idx = base + c;
                if idx >= self.data.len() {
                    break;
                }
                let b = self.data[idx];
                let color = class_color(classify(b), p);
                let hx = hex_col_char(c);

                // Hover highlight behind both panes for this byte.
                if self.hovered == Some(idx) {
                    scene.rounded_rect(
                        Rect::from_xywh(
                            inner_left + hx as f32 * char_w - char_w * 0.15,
                            y,
                            char_w * 2.3,
                            row_h,
                        ),
                        p.selection,
                        cx.theme.radius.sm,
                    );
                    if ascii {
                        scene.rounded_rect(
                            Rect::from_xywh(
                                inner_left + (ASCII_START + c) as f32 * char_w - char_w * 0.1,
                                y,
                                char_w * 1.2,
                                row_h,
                            ),
                            p.selection,
                            cx.theme.radius.sm,
                        );
                    }
                }

                mono(scene, hx, y, format!("{b:02X}"), color);

                if ascii {
                    let ch = if (0x20..=0x7e).contains(&b) {
                        b as char
                    } else {
                        '.'
                    };
                    let ascii_color = if ch == '.' { p.text_muted } else { color };
                    mono(scene, ASCII_START + c, y, ch.to_string(), ascii_color);
                }
            }
        }

        scene.pop_clip();
    }

    fn event(&mut self, cx: &mut EventCx<'_>, bounds: Rect, event: &InputEvent) {
        let pad = cx.theme.spacing.md;
        match event {
            InputEvent::PointerMoved { pos } => {
                self.hovered = if bounds.contains(*pos) {
                    self.byte_at(cx.fonts, bounds, *pos, pad)
                } else {
                    None
                };
            }
            InputEvent::PointerLeft => self.hovered = None,
            InputEvent::Scroll { pos, delta } => {
                if bounds.contains(*pos) {
                    let step = delta.y.round() as i64;
                    let new_top = (self.top_row as i64 - step).clamp(0, self.max_top() as i64);
                    self.top_row = new_top as usize;
                }
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn byte_classification() {
        assert_eq!(classify(0x00), ByteClass::Zero);
        assert_eq!(classify(b'A'), ByteClass::Printable);
        assert_eq!(classify(0x20), ByteClass::Printable);
        assert_eq!(classify(0x7e), ByteClass::Printable);
        assert_eq!(classify(0x7f), ByteClass::Control);
        assert_eq!(classify(0x0a), ByteClass::Control);
        assert_eq!(classify(0x80), ByteClass::High);
        assert_eq!(classify(0xff), ByteClass::High);
    }

    #[test]
    fn column_grid_positions() {
        // Offset is 8 chars; hex starts at 10.
        assert_eq!(hex_col_char(0), 10);
        assert_eq!(hex_col_char(1), 13);
        // The 8-byte group gap adds one extra char from column 8 on.
        assert_eq!(hex_col_char(7), 10 + 21);
        assert_eq!(hex_col_char(8), 10 + 24 + 1);
        // ASCII pane sits past the hex area.
        assert!(ASCII_START > hex_col_char(15) + 2);
        assert_eq!(TOTAL_CHARS, ASCII_START + 16);
    }

    #[test]
    fn scroll_bounds() {
        let view = HexView::new(vec![0u8; 16 * 100]).rows(16);
        assert_eq!(view.total_rows(), 100);
        assert_eq!(view.max_top(), 84);

        let small = HexView::new(vec![0u8; 16 * 4]).rows(16);
        assert_eq!(small.max_top(), 0); // nothing to scroll
    }
}
