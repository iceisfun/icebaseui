//! [`TextBox`] — a single-line editable text field bound to a `Signal<String>`.
//!
//! Supports click-to-focus, a caret, selection (drag or Shift+arrows),
//! insert/delete/navigation, Select-All (Ctrl+A), clipboard cut/copy/paste
//! (Ctrl+X/C/V), horizontal scrolling to keep the caret visible, a placeholder,
//! and a password mode. Keyboard focus is arbitrated through [`crate::focus`];
//! the [`App`](crate::App) suppresses plain-key global shortcuts while a field
//! is focused so typing works.
//!
//! Caret geometry uses the same per-glyph advances the renderer lays text out
//! with, so the caret and hit-testing line up with what is drawn.

use baseui_core::paint::{RectShape, Scene};
use baseui_core::{Id, Insets, Point, Rect, Signal, Size};

use super::{EventCx, LayoutCx, PaintCx, Widget};
use crate::event::{InputEvent, Key, Modifiers, PointerButton};
use crate::focus;
use crate::layout::Constraints;
use crate::text::FontId;

const BULLET: char = '\u{2022}';

type SubmitFn = Box<dyn FnMut(&str)>;

/// A single-line editable text field.
pub struct TextBox {
    value: Signal<String>,
    id: Id,
    /// Caret position as a char index.
    caret: usize,
    /// Selection anchor (char index); a selection exists when it differs from
    /// the caret.
    anchor: Option<usize>,
    dragging: bool,
    scroll_x: f32,
    width: Option<f32>,
    font_size: f32,
    password: bool,
    placeholder: String,
    on_enter: Option<SubmitFn>,
    hovered: bool,
}

impl TextBox {
    pub fn new(value: Signal<String>) -> Self {
        TextBox {
            value,
            id: Id::next(),
            caret: 0,
            anchor: None,
            dragging: false,
            scroll_x: 0.0,
            width: None,
            font_size: 14.0,
            password: false,
            placeholder: String::new(),
            on_enter: None,
            hovered: false,
        }
    }

    /// Fix the field width (default: fill the available width).
    pub fn width(mut self, width: f32) -> Self {
        self.width = Some(width);
        self
    }

    /// Grey hint shown when the field is empty.
    pub fn placeholder(mut self, text: impl Into<String>) -> Self {
        self.placeholder = text.into();
        self
    }

    /// Render the text as bullets and disable copy/cut.
    pub fn password(mut self) -> Self {
        self.password = true;
        self
    }

    /// Called with the current value when Enter is pressed.
    pub fn on_enter(mut self, f: impl FnMut(&str) + 'static) -> Self {
        self.on_enter = Some(Box::new(f));
        self
    }

    // -- text helpers ------------------------------------------------------

    /// What is actually drawn (bullets for a password field).
    fn display(&self) -> String {
        let value = self.value.get();
        if self.password {
            BULLET.to_string().repeat(value.chars().count())
        } else {
            value
        }
    }

    fn len(&self) -> usize {
        self.value.get().chars().count()
    }

    fn sel_range(&self) -> Option<(usize, usize)> {
        self.anchor.and_then(|a| {
            if a == self.caret {
                None
            } else {
                Some((a.min(self.caret), a.max(self.caret)))
            }
        })
    }

    fn move_caret(&mut self, to: usize, extend: bool) {
        if extend {
            if self.anchor.is_none() {
                self.anchor = Some(self.caret);
            }
        } else {
            self.anchor = None;
        }
        self.caret = to;
    }

    /// Remove the current selection from `s`, updating the caret. Returns
    /// whether anything was removed.
    fn delete_selection(&mut self, s: &mut String) -> bool {
        if let Some((a, b)) = self.sel_range() {
            s.replace_range(char_byte(s, a)..char_byte(s, b), "");
            self.caret = a;
            self.anchor = None;
            true
        } else {
            false
        }
    }

    fn insert(&mut self, text: &str) {
        let mut s = self.value.get();
        self.delete_selection(&mut s);
        let byte = char_byte(&s, self.caret);
        s.insert_str(byte, text);
        self.caret += text.chars().count();
        self.value.set(s);
    }

    fn backspace(&mut self) {
        let mut s = self.value.get();
        if !self.delete_selection(&mut s) && self.caret > 0 {
            s.replace_range(char_byte(&s, self.caret - 1)..char_byte(&s, self.caret), "");
            self.caret -= 1;
        }
        self.value.set(s);
    }

    fn delete(&mut self) {
        let mut s = self.value.get();
        if !self.delete_selection(&mut s) && self.caret < s.chars().count() {
            s.replace_range(char_byte(&s, self.caret)..char_byte(&s, self.caret + 1), "");
        }
        self.value.set(s);
    }

    fn selected_text(&self) -> String {
        if let Some((a, b)) = self.sel_range() {
            let s = self.value.get();
            s[char_byte(&s, a)..char_byte(&s, b)].to_string()
        } else {
            String::new()
        }
    }

    fn copy(&self) {
        if self.password {
            return;
        }
        let text = self.selected_text();
        if !text.is_empty() {
            crate::clipboard::set_text(&text);
        }
    }

    fn cut(&mut self) {
        if self.password {
            return;
        }
        self.copy();
        let mut s = self.value.get();
        if self.delete_selection(&mut s) {
            self.value.set(s);
        }
    }

    fn paste(&mut self) {
        if let Some(text) = crate::clipboard::get_text() {
            let clean: String = text.chars().filter(|c| !c.is_control()).collect();
            if !clean.is_empty() {
                self.insert(&clean);
            }
        }
    }

    fn handle_key(&mut self, key: &Key, mods: Modifiers) {
        let len = self.len();
        let shift = mods.shift;
        let ctrl = mods.ctrl || mods.meta;
        match key {
            Key::Left => self.move_caret(self.caret.saturating_sub(1), shift),
            Key::Right => self.move_caret((self.caret + 1).min(len), shift),
            Key::Home => self.move_caret(0, shift),
            Key::End => self.move_caret(len, shift),
            Key::Backspace => self.backspace(),
            Key::Delete => self.delete(),
            Key::Escape => focus::clear(),
            Key::Enter => {
                if let Some(cb) = self.on_enter.as_mut() {
                    let value = self.value.get();
                    cb(&value);
                }
                focus::clear();
            }
            Key::Character(c) if ctrl => match c.to_ascii_lowercase() {
                'a' => {
                    self.anchor = Some(0);
                    self.caret = len;
                }
                'c' => self.copy(),
                'x' => self.cut(),
                'v' => self.paste(),
                _ => {}
            },
            _ => {}
        }
    }

    fn on_text(&mut self, text: &str) {
        let clean: String = text.chars().filter(|c| !c.is_control()).collect();
        if !clean.is_empty() {
            self.insert(&clean);
        }
    }

    fn inner_rect(&self, bounds: Rect, cx_theme: &crate::theme::Theme) -> Rect {
        bounds.shrink(Insets::symmetric(cx_theme.spacing.md, cx_theme.spacing.sm))
    }

    /// Lay out the displayed text. Every caret/selection/hit-test question is a
    /// lookup on this — see [`crate::text::Line`].
    fn line(&self, fonts: &crate::text::Fonts) -> crate::text::Line {
        fonts.layout_line(&self.display(), self.font_size, FontId::Ui)
    }

    /// Char index nearest to pointer x within `inner`.
    fn hit(&self, fonts: &crate::text::Fonts, inner_left: f32, x: f32) -> usize {
        self.line(fonts).col_at(x - inner_left + self.scroll_x)
    }
}

impl Widget for TextBox {
    fn layout(&mut self, cx: &mut LayoutCx<'_>, constraints: Constraints) -> Size {
        let line_h = cx.fonts.line_height(self.font_size, FontId::Ui);
        let h = line_h + cx.theme.spacing.sm * 2.0 + 2.0;
        let w = self.width.unwrap_or_else(|| {
            if constraints.max.width.is_finite() {
                constraints.max.width
            } else {
                180.0
            }
        });
        constraints.constrain(Size::new(w, h))
    }

    fn paint(&mut self, cx: &mut PaintCx<'_>, bounds: Rect, scene: &mut Scene) {
        let p = &cx.theme.palette;
        let focused = focus::has(self.id);

        let border = if focused {
            p.accent
        } else if self.hovered {
            p.text_muted
        } else {
            p.border
        };
        scene.push_rect(
            RectShape::fill(bounds, p.surface_variant)
                .with_corner_radius(cx.theme.radius.sm)
                .with_border(1.0, border),
        );

        let inner = self.inner_rect(bounds, cx.theme);
        scene.push_clip(inner);

        let value = self.value.get();
        let display = self.display();
        let line_h = cx.fonts.line_height(self.font_size, FontId::Ui);
        let ty = inner.top() + (inner.height() - line_h) * 0.5;

        // Keep the caret within the viewport.
        let line = cx.fonts.layout_line(&display, self.font_size, FontId::Ui);
        let caret_x = line.x_of(self.caret);
        let inner_w = inner.width();
        if caret_x - self.scroll_x > inner_w - 2.0 {
            self.scroll_x = caret_x - inner_w + 2.0;
        }
        if caret_x - self.scroll_x < 0.0 {
            self.scroll_x = caret_x;
        }
        self.scroll_x = self.scroll_x.max(0.0);
        let text_x = inner.left() - self.scroll_x;

        // Selection highlight.
        if let Some((a, b)) = self.sel_range() {
            let (ax, bx) = line.span(a, b);
            scene.rect(
                Rect::from_xywh(text_x + ax, ty, (bx - ax).max(1.0), line_h),
                p.selection,
            );
        }

        // Placeholder or text.
        if value.is_empty() && !self.placeholder.is_empty() {
            scene.text(
                Point::new(inner.left(), ty),
                self.placeholder.clone(),
                self.font_size,
                p.text_muted,
            );
        } else {
            scene.text(Point::new(text_x, ty), display, self.font_size, p.text);
        }

        // Caret.
        if focused {
            scene.rect(Rect::from_xywh(text_x + caret_x, ty, 1.5, line_h), p.text);
        }

        scene.pop_clip();
    }

    fn event(&mut self, cx: &mut EventCx<'_>, bounds: Rect, event: &InputEvent) {
        let inner = self.inner_rect(bounds, cx.theme);
        match event {
            InputEvent::PointerMoved { pos } => {
                self.hovered = bounds.contains(*pos);
                if self.dragging {
                    self.caret = self.hit(cx.fonts, inner.left(), pos.x);
                }
            }
            InputEvent::PointerLeft => self.hovered = false,
            InputEvent::PointerPressed {
                pos,
                button: PointerButton::Primary,
            } => {
                if bounds.contains(*pos) {
                    focus::set(self.id);
                    let idx = self.hit(cx.fonts, inner.left(), pos.x);
                    self.caret = idx;
                    self.anchor = Some(idx);
                    self.dragging = true;
                    cx.consume();
                } else if focus::has(self.id) {
                    focus::clear();
                }
            }
            InputEvent::PointerReleased {
                button: PointerButton::Primary,
                ..
            } => {
                self.dragging = false;
                if self.anchor == Some(self.caret) {
                    self.anchor = None;
                }
            }
            InputEvent::Key {
                key,
                pressed: true,
                mods,
            } => {
                if focus::has(self.id) {
                    self.handle_key(key, *mods);
                }
            }
            InputEvent::Text { text } => {
                if focus::has(self.id) {
                    self.on_text(text);
                }
            }
            _ => {}
        }
    }
}

/// Byte offset of char index `i` in `s` (clamped to the end).
fn char_byte(s: &str, i: usize) -> usize {
    s.char_indices().nth(i).map(|(b, _)| b).unwrap_or(s.len())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::text::Fonts;
    use crate::theme::Theme;
    use baseui_core::create_signal;

    fn ecx<'a>(fonts: &'a Fonts, theme: &'a Theme) -> EventCx<'a> {
        EventCx::new(fonts, theme, Size::new(1000.0, 1000.0))
    }

    #[test]
    fn typing_inserts_and_backspace_deletes() {
        let Some(fonts) = Fonts::load() else {
            return;
        };
        let theme = Theme::dark();
        let value = create_signal(String::new());
        let mut tb = TextBox::new(value);
        focus::set(tb.id);

        let mut cx = ecx(&fonts, &theme);
        let b = Rect::from_xywh(0.0, 0.0, 200.0, 28.0);
        tb.event(
            &mut cx,
            b,
            &InputEvent::Text {
                text: "Hello".into(),
            },
        );
        assert_eq!(value.get(), "Hello");
        assert_eq!(tb.caret, 5);

        tb.event(
            &mut cx,
            b,
            &InputEvent::Key {
                key: Key::Backspace,
                pressed: true,
                mods: Modifiers::default(),
            },
        );
        assert_eq!(value.get(), "Hell");

        // Home, then delete the first char.
        tb.event(&mut cx, b, &key(Key::Home, Modifiers::default()));
        tb.event(&mut cx, b, &key(Key::Delete, Modifiers::default()));
        assert_eq!(value.get(), "ell");
    }

    #[test]
    fn select_all_then_type_replaces() {
        let Some(fonts) = Fonts::load() else {
            return;
        };
        let theme = Theme::dark();
        let value = create_signal(String::from("abc"));
        let mut tb = TextBox::new(value);
        tb.caret = 3;
        focus::set(tb.id);

        let mut cx = ecx(&fonts, &theme);
        let b = Rect::from_xywh(0.0, 0.0, 200.0, 28.0);
        let ctrl = Modifiers {
            ctrl: true,
            ..Default::default()
        };
        tb.event(&mut cx, b, &key(Key::Character('a'), ctrl));
        assert_eq!(tb.sel_range(), Some((0, 3)));
        tb.event(&mut cx, b, &InputEvent::Text { text: "Z".into() });
        assert_eq!(value.get(), "Z");
    }

    fn key(key: Key, mods: Modifiers) -> InputEvent {
        InputEvent::Key {
            key,
            pressed: true,
            mods,
        }
    }
}
