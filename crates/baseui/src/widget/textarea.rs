//! [`TextArea`] — a multi-line text editor: the code/log/script area.
//!
//! # Why this is affordable
//!
//! The expensive part of rich text is **wrapping with proportional fonts** — that
//! needs the shaping/galley engine sketched in `docs/rich-text.md`. A code area
//! needs none of it. Because lines are **not wrapped**, every position is a
//! prefix sum of glyph advances along one line, so the caret, hit-testing, and
//! the x of any coloured run are all the same cheap computation. (With a
//! monospace font it degenerates to `col × char_width`.)
//!
//! That buys, at very little cost:
//!
//! - **syntax colouring** — a [`Highlighter`] returns [`Span`]s per line, drawn as
//!   separate coloured runs (the same trick [`HexView`](super::HexView) uses),
//! - **diagnostics** — squiggly underlines under a range, via the `Squiggle`
//!   decoration primitive,
//! - **line numbers**, current-line highlight, and selection,
//! - **undo/redo** — opt in with [`TextArea::undo_history`], or inject your own
//!   [`History`](crate::undo::History) implementation with [`TextArea::history`]
//!   when whole-document snapshots are too heavy for your document.
//!
//! Only the *visible* lines are laid out and painted, so a long document costs
//! what fits on screen.
//!
//! Deliberately **not** supported: soft wrapping. That is the galley engine's
//! job, and pretending otherwise would break the prefix-sum invariant everything
//! here relies on.

use baseui_core::paint::{RectShape, Scene};
use baseui_core::{Color, Id, Point, Rect, Size};

use super::{EventCx, LayoutCx, PaintCx, Widget};
use crate::event::{InputEvent, Key, Modifiers, PointerButton};
use crate::focus;
use crate::layout::Constraints;
use crate::text::FontId;
use crate::undo::{EditKind, History, Snapshot, UndoStack};

/// A coloured run within one line, in **character** columns.
#[derive(Clone, Copy, Debug)]
pub struct Span {
    /// First column of the run.
    pub start: usize,
    /// One past the last column of the run.
    pub end: usize,
    /// Colour to draw the run in.
    pub color: Color,
}

/// Colours one line. Called only for lines that are actually visible.
pub type Highlighter = Box<dyn Fn(&str) -> Vec<Span>>;

/// A squiggly underline under a range of one line — an error or warning.
#[derive(Clone, Copy, Debug)]
pub struct Diagnostic {
    /// Line the squiggle sits under. A diagnostic never spans lines.
    pub line: usize,
    /// First column covered, in **character** columns.
    pub start: usize,
    /// One past the last column covered.
    pub end: usize,
    /// Colour of the squiggle — the caller's severity scale, not ours.
    pub color: Color,
}

/// Default row height, as a multiple of the font's natural line box.
///
/// Faces routinely report a line box of `ascent + descent` with **zero** line
/// gap, which is too tight to read as code and leaves nowhere to draw a
/// diagnostic underline that is not on top of the glyphs. Editors all add
/// leading; so do we.
pub const DEFAULT_LINE_SPACING: f32 = 1.2;

type ChangeFn = Box<dyn FnMut(&str)>;
/// Recomputes diagnostics from the whole document after every edit.
pub type Checker = Box<dyn Fn(&str) -> Vec<Diagnostic>>;

/// A multi-line text editor.
///
/// It **owns** its buffer rather than binding a `Signal<String>`: a signal would
/// clone the whole document on every read, which is fine for a one-line
/// [`TextBox`](super::TextBox) and wasteful here. Observe edits with
/// [`TextArea::on_change`] (and set a signal there if you want reactivity).
pub struct TextArea {
    lines: Vec<String>,
    id: Id,
    /// Caret as (line, column-in-chars).
    caret: (usize, usize),
    /// Selection anchor; a selection exists when it differs from the caret.
    anchor: Option<(usize, usize)>,
    dragging: bool,
    scroll_x: f32,
    scroll_y: f32,
    font_size: f32,
    font: FontId,
    /// Row height as a multiple of the font's natural line box.
    line_spacing: f32,
    line_numbers: bool,
    read_only: bool,
    highlighter: Option<Highlighter>,
    diagnostics: Vec<Diagnostic>,
    checker: Option<Checker>,
    on_change: Option<ChangeFn>,
    /// Undo/redo, if enabled. `None` means edits are not recorded at all — a
    /// read-only log view has no use for a history.
    history: Option<Box<dyn History>>,
    hovered: bool,
}

impl TextArea {
    /// An editable area holding `text`, split into lines on `\n`.
    pub fn new(text: impl AsRef<str>) -> Self {
        TextArea {
            lines: split_lines(text.as_ref()),
            id: Id::next(),
            caret: (0, 0),
            anchor: None,
            dragging: false,
            scroll_x: 0.0,
            scroll_y: 0.0,
            font_size: 13.0,
            font: FontId::Mono,
            line_spacing: DEFAULT_LINE_SPACING,
            line_numbers: false,
            read_only: false,
            highlighter: None,
            diagnostics: Vec::new(),
            checker: None,
            on_change: None,
            history: None,
            hovered: false,
        }
    }

    /// Glyph size in logical pixels; the row height follows from it via
    /// [`TextArea::line_spacing`].
    pub fn font_size(mut self, size: f32) -> Self {
        self.font_size = size;
        self
    }

    /// Row height as a multiple of the font's natural line box (default
    /// [`DEFAULT_LINE_SPACING`]). `1.0` packs lines as tightly as the face allows
    /// — which usually leaves no room under the text for a squiggle.
    pub fn line_spacing(mut self, factor: f32) -> Self {
        self.line_spacing = factor.max(1.0);
        self
    }

    /// Use the proportional UI font instead of monospace.
    pub fn proportional(mut self) -> Self {
        self.font = FontId::Ui;
        self
    }

    /// Show a line-number gutter.
    pub fn line_numbers(mut self) -> Self {
        self.line_numbers = true;
        self
    }

    /// Display only — no editing (still selectable and copyable).
    pub fn read_only(mut self) -> Self {
        self.read_only = true;
        self
    }

    /// Colour each line's tokens.
    pub fn highlighter(mut self, f: impl Fn(&str) -> Vec<Span> + 'static) -> Self {
        self.highlighter = Some(Box::new(f));
        self
    }

    /// Static squiggly underlines (errors/warnings).
    pub fn diagnostics(mut self, diagnostics: Vec<Diagnostic>) -> Self {
        self.diagnostics = diagnostics;
        self
    }

    /// Recompute diagnostics from the document after every edit — the seam for a
    /// linter, a parser, or a language server's replies.
    ///
    /// Runs once immediately so the initial text is checked too.
    pub fn checker(mut self, f: impl Fn(&str) -> Vec<Diagnostic> + 'static) -> Self {
        self.diagnostics = f(&self.text());
        self.checker = Some(Box::new(f));
        self
    }

    /// Replace the diagnostics imperatively (e.g. when an async check returns).
    pub fn set_diagnostics(&mut self, diagnostics: Vec<Diagnostic>) {
        self.diagnostics = diagnostics;
    }

    /// Replace the whole document, resetting the caret.
    pub fn set_text(&mut self, text: impl AsRef<str>) {
        self.lines = split_lines(text.as_ref());
        self.caret = (0, 0);
        self.anchor = None;
        let checked = self.checker.as_ref().map(|f| f(&self.text()));
        if let Some(d) = checked {
            self.diagnostics = d;
        }
    }

    /// Enable undo/redo with the built-in [`UndoStack`] — Ctrl+Z, and Ctrl+Shift+Z
    /// or Ctrl+Y to redo.
    ///
    /// The built-in keeps whole-document snapshots, which is simple and correct
    /// for source files, scripts, and config. If your document is large enough
    /// that copying it per edit hurts, inject your own with [`Self::history`].
    pub fn undo_history(mut self) -> Self {
        self.history = Some(Box::new(UndoStack::new()));
        self
    }

    /// Inject a [`History`] implementation — a piece table, a rope, an operation
    /// log, whatever fits your document.
    pub fn history(mut self, history: impl History + 'static) -> Self {
        self.history = Some(Box::new(history));
        self
    }

    /// Whether there is anything to undo / redo — for enabling menu items.
    pub fn can_undo(&self) -> bool {
        self.history.as_ref().is_some_and(|h| h.can_undo())
    }

    /// Whether there is anything to redo. `false` when no history is enabled.
    pub fn can_redo(&self) -> bool {
        self.history.as_ref().is_some_and(|h| h.can_redo())
    }

    /// Called with the whole document after every edit.
    pub fn on_change(mut self, f: impl FnMut(&str) + 'static) -> Self {
        self.on_change = Some(Box::new(f));
        self
    }

    /// The current document.
    pub fn text(&self) -> String {
        self.lines.join("\n")
    }

    // -- buffer helpers ----------------------------------------------------

    fn line_len(&self, line: usize) -> usize {
        self.lines.get(line).map(|l| l.chars().count()).unwrap_or(0)
    }

    fn clamp(&self, pos: (usize, usize)) -> (usize, usize) {
        let line = pos.0.min(self.lines.len().saturating_sub(1));
        (line, pos.1.min(self.line_len(line)))
    }

    /// Selection as an ordered (start, end) pair, if any.
    fn selection(&self) -> Option<((usize, usize), (usize, usize))> {
        let a = self.anchor?;
        if a == self.caret {
            return None;
        }
        Some(if a < self.caret {
            (a, self.caret)
        } else {
            (self.caret, a)
        })
    }

    fn move_caret(&mut self, to: (usize, usize), extend: bool) {
        // Typing, moving, then typing again must be two undo steps, not one lump
        // spanning both places.
        self.close_group();
        if extend {
            if self.anchor.is_none() {
                self.anchor = Some(self.caret);
            }
        } else {
            self.anchor = None;
        }
        self.caret = self.clamp(to);
    }

    // -- undo/redo ---------------------------------------------------------

    fn snapshot(&self) -> Snapshot {
        Snapshot {
            text: self.text(),
            caret: self.caret,
            anchor: self.anchor,
        }
    }

    /// Record the state *before* an edit. Called by every mutating operation, so
    /// there is exactly one place an edit can be forgotten from.
    fn record(&mut self, kind: EditKind) {
        if let Some(history) = self.history.as_mut() {
            let before = Snapshot {
                text: self.lines.join("\n"),
                caret: self.caret,
                anchor: self.anchor,
            };
            history.record(before, kind);
        }
    }

    /// End the coalescing group: the caret moved, so the next edit is a new step
    /// rather than a continuation of the last one.
    fn close_group(&mut self) {
        if let Some(history) = self.history.as_mut() {
            history.close_group();
        }
    }

    fn restore(&mut self, snapshot: Snapshot) {
        self.lines = split_lines(&snapshot.text);
        self.caret = self.clamp(snapshot.caret);
        self.anchor = snapshot.anchor;
        // Re-check and notify: an undone document is a changed document. It is
        // *not* re-recorded — the history moved us here, it already knows.
        self.changed();
    }

    fn undo(&mut self) {
        if self.read_only {
            return;
        }
        let current = self.snapshot();
        let restored = self.history.as_mut().and_then(|h| h.undo(current));
        if let Some(snapshot) = restored {
            self.restore(snapshot);
        }
    }

    fn redo(&mut self) {
        if self.read_only {
            return;
        }
        let current = self.snapshot();
        let restored = self.history.as_mut().and_then(|h| h.redo(current));
        if let Some(snapshot) = restored {
            self.restore(snapshot);
        }
    }

    // -- word boundaries ---------------------------------------------------

    /// The start of the word at or before `col` — where Ctrl+Left lands.
    ///
    /// Skips any whitespace immediately behind the caret first, so Ctrl+Left from
    /// the start of a word jumps over the gap to the previous word rather than
    /// stalling on the space.
    fn word_start(&self, line: usize, col: usize) -> usize {
        let chars: Vec<char> = self.lines[line].chars().collect();
        let mut i = col.min(chars.len());
        while i > 0 && chars[i - 1].is_whitespace() {
            i -= 1;
        }
        while i > 0 && !chars[i - 1].is_whitespace() {
            i -= 1;
        }
        i
    }

    /// Where Home lands: the first non-whitespace character, or column 0 if the
    /// caret is already there.
    ///
    /// Pressing Home in indented code should take you to the **code**, not to
    /// column 0 with the indentation in between — but column 0 still has to be
    /// reachable, so the two toggle. This is what every code editor does, and it
    /// is why Home is not simply `0`.
    fn smart_home(&self, line: usize, col: usize) -> usize {
        let indent = self.lines[line]
            .chars()
            .position(|c| !c.is_whitespace())
            .unwrap_or_else(|| self.line_len(line)); // an all-whitespace line

        if col == indent { 0 } else { indent }
    }

    /// The end of the word at or after `col` — where Ctrl+Right lands.
    fn word_end(&self, line: usize, col: usize) -> usize {
        let chars: Vec<char> = self.lines[line].chars().collect();
        let mut i = col.min(chars.len());
        while i < chars.len() && chars[i].is_whitespace() {
            i += 1;
        }
        while i < chars.len() && !chars[i].is_whitespace() {
            i += 1;
        }
        i
    }

    /// Called after every mutation: re-check, then notify.
    fn changed(&mut self) {
        let text = self.text();
        // Compute first, then assign — otherwise the borrow of `self.checker`
        // would still be live across the write to `self.diagnostics`.
        let checked = self.checker.as_ref().map(|f| f(&text));
        if let Some(d) = checked {
            self.diagnostics = d;
        }
        if let Some(cb) = self.on_change.as_mut() {
            cb(&text);
        }
    }

    fn delete_selection(&mut self) -> bool {
        let Some((start, end)) = self.selection() else {
            return false;
        };
        let head: String = self.lines[start.0].chars().take(start.1).collect();
        let tail: String = self.lines[end.0].chars().skip(end.1).collect();
        self.lines
            .splice(start.0..=end.0, [format!("{head}{tail}")]);
        self.caret = start;
        self.anchor = None;
        true
    }

    fn insert(&mut self, text: &str) {
        if self.read_only {
            return;
        }
        // Classify the edit so the history can group it sensibly: a single
        // ordinary character continues a word, whitespace ends one, and anything
        // bulkier (a paste, an autocompletion) is one user action on its own.
        // Replacing a selection is never a continuation of the typing before it.
        let kind = if self.selection().is_some() || text.chars().count() > 1 {
            EditKind::Paste
        } else if text.chars().next().is_some_and(char::is_whitespace) {
            EditKind::Break
        } else {
            EditKind::Typing
        };
        self.record(kind);

        self.delete_selection();
        let (line, col) = self.caret;
        let current = &self.lines[line];
        let head: String = current.chars().take(col).collect();
        let tail: String = current.chars().skip(col).collect();

        let inserted = split_lines(text);
        if inserted.len() == 1 {
            self.lines[line] = format!("{head}{}{tail}", inserted[0]);
            self.caret = (line, col + inserted[0].chars().count());
        } else {
            let last = inserted.len() - 1;
            let mut new_lines: Vec<String> = Vec::with_capacity(inserted.len());
            new_lines.push(format!("{head}{}", inserted[0]));
            for l in &inserted[1..last] {
                new_lines.push(l.clone());
            }
            new_lines.push(format!("{}{tail}", inserted[last]));
            let caret_col = inserted[last].chars().count();
            self.lines.splice(line..=line, new_lines);
            self.caret = (line + last, caret_col);
        }
        self.anchor = None;
        self.changed();
    }

    fn backspace(&mut self) {
        if self.read_only {
            return;
        }
        self.record(if self.selection().is_some() {
            EditKind::Paste // deleting a selection is one action, not a run
        } else {
            EditKind::Delete
        });
        if self.delete_selection() {
            self.changed();
            return;
        }
        let (line, col) = self.caret;
        if col > 0 {
            let mut chars: Vec<char> = self.lines[line].chars().collect();
            chars.remove(col - 1);
            self.lines[line] = chars.into_iter().collect();
            self.caret = (line, col - 1);
        } else if line > 0 {
            // Join with the previous line.
            let prev_len = self.line_len(line - 1);
            let tail = self.lines.remove(line);
            self.lines[line - 1].push_str(&tail);
            self.caret = (line - 1, prev_len);
        }
        self.changed();
    }

    fn delete(&mut self) {
        if self.read_only {
            return;
        }
        self.record(if self.selection().is_some() {
            EditKind::Paste
        } else {
            EditKind::Delete
        });
        if self.delete_selection() {
            self.changed();
            return;
        }
        let (line, col) = self.caret;
        if col < self.line_len(line) {
            let mut chars: Vec<char> = self.lines[line].chars().collect();
            chars.remove(col);
            self.lines[line] = chars.into_iter().collect();
        } else if line + 1 < self.lines.len() {
            let next = self.lines.remove(line + 1);
            self.lines[line].push_str(&next);
        }
        self.changed();
    }

    fn selected_text(&self) -> String {
        let Some((start, end)) = self.selection() else {
            return String::new();
        };
        if start.0 == end.0 {
            return self.lines[start.0]
                .chars()
                .skip(start.1)
                .take(end.1 - start.1)
                .collect();
        }
        let mut out = String::new();
        out.extend(self.lines[start.0].chars().skip(start.1));
        for line in &self.lines[start.0 + 1..end.0] {
            out.push('\n');
            out.push_str(line);
        }
        out.push('\n');
        out.extend(self.lines[end.0].chars().take(end.1));
        out
    }

    // -- geometry ----------------------------------------------------------

    /// The height of one row: the font's natural line box times [`Self::line_spacing`].
    ///
    /// Glyphs are drawn at the **top** of the row, so the extra leading lands
    /// underneath them — which is where the descenders and the diagnostic
    /// squiggles have to live. A face's natural line box is often just
    /// `ascent + descent` with no gap at all, leaving literally nowhere to put an
    /// underline that is not on top of the text.
    fn line_height(&self, cx_fonts: &crate::text::Fonts) -> f32 {
        cx_fonts.line_height(self.font_size, self.font) * self.line_spacing
    }

    /// The band a squiggle occupies under a run of text, relative to the row top.
    ///
    /// Anchored to the **baseline**, not to the bottom of the row: the row bottom
    /// is a different distance below the text for every face and size, so a
    /// squiggle placed there lands on the glyphs for some of them. Start half a
    /// descent below the baseline — clear of every glyph that has no descender,
    /// and only grazing the tails of `p`/`g`/`y`, which is what editors do.
    fn squiggle_band(&self, cx_fonts: &crate::text::Fonts) -> (f32, f32) {
        let m = cx_fonts.metrics(self.font_size, self.font);
        let top = m.ascent + m.descent * 0.5;
        // Enough amplitude to read as a wave, but never past the row.
        let height = (4.5 * crate::text::scale()).min(self.line_height(cx_fonts) - top);
        (top, height.max(3.0))
    }

    fn gutter_width(&self, cx_fonts: &crate::text::Fonts) -> f32 {
        if !self.line_numbers {
            return 0.0;
        }
        let digits = self.lines.len().max(1).to_string().len().max(2);
        let w = cx_fonts.char_advance('0', self.font_size, self.font);
        w * digits as f32 + 16.0 * crate::text::scale()
    }

    /// Lay out one line: cumulative x offsets for every column boundary. The
    /// caret, hit-testing, selection rects, and every coloured run's x are all
    /// lookups on this — see [`crate::text::Line`].
    fn line(&self, cx_fonts: &crate::text::Fonts, index: usize) -> crate::text::Line {
        let text = self.lines.get(index).map(String::as_str).unwrap_or("");
        cx_fonts.layout_line(text, self.font_size, self.font)
    }

    /// The (line, col) under a pointer position.
    fn pos_at(&self, cx_fonts: &crate::text::Fonts, bounds: Rect, pos: Point) -> (usize, usize) {
        let line_h = self.line_height(cx_fonts);
        let gutter = self.gutter_width(cx_fonts);
        let y = pos.y - bounds.top() + self.scroll_y;
        let index =
            ((y / line_h).floor().max(0.0) as usize).min(self.lines.len().saturating_sub(1));
        let x = pos.x - bounds.left() - gutter + self.scroll_x;
        (index, self.line(cx_fonts, index).col_at(x.max(0.0)))
    }

    // -- editing keys ------------------------------------------------------

    fn handle_key(&mut self, key: &Key, mods: Modifiers) {
        let shift = mods.shift;
        let ctrl = mods.ctrl || mods.meta;
        let (line, col) = self.caret;

        match key {
            // Ctrl+arrow jumps a word; plain arrow steps a character and wraps
            // over the line break, so Left at column 0 lands at the end of the
            // previous line rather than doing nothing.
            Key::Left if ctrl => self.move_caret((line, self.word_start(line, col)), shift),
            Key::Right if ctrl => self.move_caret((line, self.word_end(line, col)), shift),
            Key::Left => {
                if col > 0 {
                    self.move_caret((line, col - 1), shift);
                } else if line > 0 {
                    self.move_caret((line - 1, self.line_len(line - 1)), shift);
                }
            }
            Key::Right => {
                if col < self.line_len(line) {
                    self.move_caret((line, col + 1), shift);
                } else if line + 1 < self.lines.len() {
                    self.move_caret((line + 1, 0), shift);
                }
            }
            Key::Up => {
                if line > 0 {
                    self.move_caret((line - 1, col), shift);
                }
            }
            Key::Down => {
                if line + 1 < self.lines.len() {
                    self.move_caret((line + 1, col), shift);
                }
            }
            // Ctrl+Home/End go to the ends of the *document*; bare Home/End to the
            // ends of the line.
            Key::Home if ctrl => self.move_caret((0, 0), shift),
            Key::End if ctrl => {
                let last = self.lines.len().saturating_sub(1);
                self.move_caret((last, self.line_len(last)), shift);
            }
            Key::Home => self.move_caret((line, self.smart_home(line, col)), shift),
            Key::End => self.move_caret((line, self.line_len(line)), shift),
            Key::PageUp => self.move_caret((line.saturating_sub(20), col), shift),
            Key::PageDown => self.move_caret((line + 20, col), shift),
            Key::Backspace => self.backspace(),
            Key::Delete => self.delete(),
            Key::Enter => self.insert("\n"),
            Key::Tab => self.insert("    "),
            Key::Escape => focus::clear(),
            Key::Character(c) if ctrl => match c.to_ascii_lowercase() {
                'a' => {
                    self.anchor = Some((0, 0));
                    let last = self.lines.len().saturating_sub(1);
                    self.caret = (last, self.line_len(last));
                }
                'c' => {
                    let text = self.selected_text();
                    if !text.is_empty() {
                        crate::clipboard::set_text(&text);
                    }
                }
                'x' => {
                    let text = self.selected_text();
                    if !text.is_empty() && !self.read_only {
                        crate::clipboard::set_text(&text);
                        self.record(EditKind::Paste); // a cut is one action
                        self.delete_selection();
                        self.changed();
                    }
                }
                'v' => {
                    if let Some(text) = crate::clipboard::get_text() {
                        self.insert(&text);
                    }
                }
                // Ctrl+Shift+Z and Ctrl+Y both redo: the two conventions users
                // arrive with, and there is no cost to honouring both.
                'z' if shift => self.redo(),
                'z' => self.undo(),
                'y' => self.redo(),
                _ => {}
            },
            _ => {}
        }
    }
}

/// Split into lines, always yielding at least one (an empty document is one
/// empty line, so the caret always has somewhere to live).
fn split_lines(text: &str) -> Vec<String> {
    let mut lines: Vec<String> = text
        .replace('\r', "")
        .split('\n')
        .map(String::from)
        .collect();
    if lines.is_empty() {
        lines.push(String::new());
    }
    lines
}

/// Turn a line's spans into contiguous coloured runs covering the whole line.
fn runs(line_len: usize, spans: &[Span], default: Color) -> Vec<(usize, usize, Color)> {
    let mut spans: Vec<&Span> = spans.iter().filter(|s| s.start < s.end).collect();
    spans.sort_by_key(|s| s.start);

    let mut out = Vec::new();
    let mut cursor = 0usize;
    for span in spans {
        let start = span.start.min(line_len);
        let end = span.end.min(line_len);
        if start >= end || start < cursor {
            continue; // out of range, or overlaps one we already emitted
        }
        if start > cursor {
            out.push((cursor, start, default));
        }
        out.push((start, end, span.color));
        cursor = end;
    }
    if cursor < line_len {
        out.push((cursor, line_len, default));
    }
    out
}

impl Widget for TextArea {
    fn layout(&mut self, _cx: &mut LayoutCx<'_>, constraints: Constraints) -> Size {
        let w = if constraints.max.width.is_finite() {
            constraints.max.width
        } else {
            480.0
        };
        let h = if constraints.max.height.is_finite() {
            constraints.max.height
        } else {
            320.0
        };
        constraints.constrain(Size::new(w, h))
    }

    fn paint(&mut self, cx: &mut PaintCx<'_>, bounds: Rect, scene: &mut Scene) {
        let p = &cx.theme.palette;
        let s = crate::text::scale();
        let focused = focus::has(self.id);
        let line_h = self.line_height(cx.fonts);
        let gutter = self.gutter_width(cx.fonts);
        let (band_top, band_h) = self.squiggle_band(cx.fonts);

        scene.push_rect(
            RectShape::fill(bounds, p.surface_variant)
                .with_corner_radius(cx.theme.radius.sm)
                .with_border(1.0, if focused { p.accent } else { p.border }),
        );

        let text_left = bounds.left() + gutter;
        let view_w = (bounds.width() - gutter).max(1.0);

        // Keep the caret in view (vertically and horizontally).
        let caret_y = self.caret.0 as f32 * line_h;
        if caret_y < self.scroll_y {
            self.scroll_y = caret_y;
        } else if caret_y + line_h > self.scroll_y + bounds.height() {
            self.scroll_y = caret_y + line_h - bounds.height();
        }
        let max_scroll_y = ((self.lines.len() as f32 * line_h) - bounds.height()).max(0.0);
        self.scroll_y = self.scroll_y.clamp(0.0, max_scroll_y);

        let caret_x = self.line(cx.fonts, self.caret.0).x_of(self.caret.1);
        if caret_x - self.scroll_x > view_w - 8.0 * s {
            self.scroll_x = caret_x - view_w + 8.0 * s;
        }
        if caret_x < self.scroll_x {
            self.scroll_x = caret_x;
        }
        self.scroll_x = self.scroll_x.max(0.0);

        scene.push_clip(bounds.shrink(baseui_core::Insets::all(1.0)));

        // Only the visible lines are touched — a long document costs what fits.
        let first = (self.scroll_y / line_h).floor().max(0.0) as usize;
        let count = (bounds.height() / line_h).ceil() as usize + 1;
        let last = (first + count).min(self.lines.len());
        let selection = self.selection();

        for i in first..last {
            let y = bounds.top() + i as f32 * line_h - self.scroll_y;
            let text = self.lines[i].clone();
            let line = cx.fonts.layout_line(&text, self.font_size, self.font);
            let len = line.len();

            // Current-line highlight.
            if focused && i == self.caret.0 && selection.is_none() {
                scene.rect(
                    Rect::from_xywh(text_left, y, view_w, line_h),
                    p.surface.with_alpha(0.5),
                );
            }

            // Selection.
            if let Some((start, end)) = selection {
                if i >= start.0 && i <= end.0 {
                    let c0 = if i == start.0 { start.1 } else { 0 };
                    let c1 = if i == end.0 { end.1 } else { len };
                    let (x0, x1) = line.span(c0.min(len), c1.min(len));
                    // A selected newline shows as a sliver past the last glyph.
                    let w = if i < end.0 {
                        (x1 - x0).max(0.0) + 4.0 * s
                    } else {
                        (x1 - x0).max(0.0)
                    };
                    scene.rect(
                        Rect::from_xywh(text_left + x0 - self.scroll_x, y, w, line_h),
                        p.selection,
                    );
                }
            }

            // Text, as coloured runs.
            let spans = self
                .highlighter
                .as_ref()
                .map(|h| h(&text))
                .unwrap_or_default();
            for (c0, c1, color) in runs(len, &spans, p.text) {
                let run: String = text.chars().skip(c0).take(c1 - c0).collect();
                if run.trim().is_empty() && spans.is_empty() {
                    continue;
                }
                scene.text_font(
                    Point::new(text_left + line.x_of(c0) - self.scroll_x, y),
                    run,
                    self.font_size,
                    color,
                    self.font,
                );
            }

            // Diagnostics: a wavy underline under the range, sitting just below
            // the baseline (see `squiggle_band`).
            for d in self.diagnostics.iter().filter(|d| d.line == i) {
                let (x0, x1) = line.span(d.start.min(len), d.end.min(len));
                if x1 > x0 {
                    scene.squiggle(
                        Rect::from_xywh(
                            text_left + x0 - self.scroll_x,
                            y + band_top,
                            x1 - x0,
                            band_h,
                        ),
                        d.color,
                    );
                }
            }
        }

        // Caret.
        if focused {
            let y = bounds.top() + self.caret.0 as f32 * line_h - self.scroll_y;
            scene.rect(
                Rect::from_xywh(text_left + caret_x - self.scroll_x, y, 1.5 * s, line_h),
                p.text,
            );
        }

        scene.pop_clip();

        // Line-number gutter, painted last so text scrolled left slides under it.
        if self.line_numbers {
            scene.rect(
                Rect::from_xywh(
                    bounds.left() + 1.0,
                    bounds.top() + 1.0,
                    gutter,
                    bounds.height() - 2.0,
                ),
                p.surface,
            );
            scene.push_clip(Rect::from_xywh(
                bounds.left(),
                bounds.top() + 1.0,
                gutter,
                bounds.height() - 2.0,
            ));
            for i in first..last {
                let y = bounds.top() + i as f32 * line_h - self.scroll_y;
                let label = (i + 1).to_string();
                let w = cx.fonts.measure(&label, self.font_size, self.font).width;
                let color = if i == self.caret.0 {
                    p.text
                } else {
                    p.text_muted
                };
                scene.text_font(
                    Point::new(bounds.left() + gutter - w - 8.0 * s, y),
                    label,
                    self.font_size,
                    color,
                    self.font,
                );
            }
            scene.pop_clip();
        }
    }

    fn event(&mut self, cx: &mut EventCx<'_>, bounds: Rect, event: &InputEvent) {
        match event {
            InputEvent::PointerMoved { pos } => {
                self.hovered = bounds.contains(*pos);
                if self.dragging {
                    self.caret = self.pos_at(cx.fonts, bounds, *pos);
                }
            }
            InputEvent::PointerLeft => self.hovered = false,
            InputEvent::PointerPressed {
                pos,
                button: PointerButton::Primary,
            } => {
                if bounds.contains(*pos) {
                    focus::set(self.id);
                    // Clicking moves the caret, so it ends the undo group just as
                    // an arrow key would — this does not go through move_caret().
                    self.close_group();
                    let at = self.pos_at(cx.fonts, bounds, *pos);
                    self.caret = at;
                    self.anchor = Some(at);
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
            InputEvent::Scroll { pos, delta } => {
                if bounds.contains(*pos) {
                    let line_h = self.line_height(cx.fonts);
                    self.scroll_y = (self.scroll_y - delta.y * line_h * 3.0).max(0.0);
                    cx.consume();
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
                if focus::has(self.id) && !self.read_only {
                    let clean: String = text.chars().filter(|c| !c.is_control()).collect();
                    if !clean.is_empty() {
                        self.insert(&clean);
                    }
                }
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::text::Fonts;
    use crate::theme::Theme;
    use baseui_core::Size;

    fn ta(text: &str) -> TextArea {
        let t = TextArea::new(text);
        focus::set(t.id);
        t
    }

    fn key(k: Key) -> InputEvent {
        InputEvent::Key {
            key: k,
            pressed: true,
            mods: Modifiers::default(),
        }
    }

    #[test]
    fn enter_splits_a_line_and_backspace_rejoins_it() {
        let Some(fonts) = Fonts::load() else { return };
        let theme = Theme::dark();
        let mut cx = EventCx::new(&fonts, &theme, Size::new(600.0, 400.0));
        let bounds = Rect::from_xywh(0.0, 0.0, 600.0, 400.0);

        let mut t = ta("hello world");
        t.caret = (0, 5);
        t.event(&mut cx, bounds, &key(Key::Enter));
        assert_eq!(t.lines, vec!["hello", " world"]);
        assert_eq!(t.caret, (1, 0));

        // Backspace at column 0 joins with the previous line.
        t.event(&mut cx, bounds, &key(Key::Backspace));
        assert_eq!(t.lines, vec!["hello world"]);
        assert_eq!(t.caret, (0, 5));
    }

    #[test]
    fn multi_line_selection_deletes_across_lines() {
        let Some(fonts) = Fonts::load() else { return };
        let theme = Theme::dark();
        let mut cx = EventCx::new(&fonts, &theme, Size::new(600.0, 400.0));
        let bounds = Rect::from_xywh(0.0, 0.0, 600.0, 400.0);

        let mut t = ta("one\ntwo\nthree");
        t.anchor = Some((0, 1)); // from "o|ne"
        t.caret = (2, 2); // to "th|ree"
        assert_eq!(t.selected_text(), "ne\ntwo\nth");

        t.event(&mut cx, bounds, &InputEvent::Text { text: "X".into() });
        assert_eq!(t.lines, vec!["oXree"]);
        assert_eq!(t.caret, (0, 2));
    }

    #[test]
    fn pasting_multi_line_text_inserts_lines() {
        let mut t = ta("ab");
        t.caret = (0, 1);
        t.insert("1\n2\n3");
        assert_eq!(t.lines, vec!["a1", "2", "3b"]);
        assert_eq!(t.caret, (2, 1));
    }

    /// The squiggle must sit *under* the text, not through it.
    ///
    /// The first attempt anchored it to the bottom of the line box, which is a
    /// different distance below the baseline for every face and size — for a
    /// typical mono face at 13px it put the wave's crest **above** the baseline,
    /// straight through the glyphs. Anchor to the baseline instead, and assert it.
    #[test]
    fn the_squiggle_sits_below_the_baseline_and_inside_the_row() {
        let Some(fonts) = Fonts::load() else { return };

        for font in [FontId::Mono, FontId::Ui] {
            for size in [11.0, 13.0, 14.0, 18.0] {
                let t = TextArea::new("x").font_size(size);
                let area = if font == FontId::Ui {
                    TextArea::new("x").font_size(size).proportional()
                } else {
                    t
                };

                let m = fonts.metrics(size, font);
                let row = area.line_height(&fonts);
                let (top, height) = area.squiggle_band(&fonts);

                assert!(
                    top >= m.ascent,
                    "{font:?}@{size}: squiggle starts {top} but the baseline is at \
                     {} -- it would cut through the glyphs",
                    m.ascent
                );
                assert!(
                    top + height <= row + 0.01,
                    "{font:?}@{size}: squiggle ends at {} but the row is only {row} tall",
                    top + height
                );
                assert!(
                    height >= 3.0,
                    "{font:?}@{size}: a {height}px wave is not legible"
                );
            }
        }
    }

    /// The default row is taller than the face's own line box — otherwise there
    /// is nowhere to put the squiggle at all.
    #[test]
    fn rows_carry_leading_below_the_text() {
        let Some(fonts) = Fonts::load() else { return };
        let area = TextArea::new("x").font_size(13.0);

        let natural = fonts.line_height(13.0, FontId::Mono);
        assert!(area.line_height(&fonts) > natural);

        // Opting out is allowed, but never below the face's own box.
        let tight = TextArea::new("x").font_size(13.0).line_spacing(0.5);
        assert!((tight.line_height(&fonts) - natural).abs() < 0.01);
    }

    fn ctrl(k: Key) -> InputEvent {
        InputEvent::Key {
            key: k,
            pressed: true,
            mods: Modifiers {
                ctrl: true,
                ..Default::default()
            },
        }
    }

    fn ctrl_shift(k: Key) -> InputEvent {
        InputEvent::Key {
            key: k,
            pressed: true,
            mods: Modifiers {
                ctrl: true,
                shift: true,
                ..Default::default()
            },
        }
    }

    fn typed(s: &str) -> InputEvent {
        InputEvent::Text { text: s.into() }
    }

    /// Undo through the real key path: typing a word, then Ctrl+Z, must take the
    /// whole word back — and Ctrl+Shift+Z must put it back.
    #[test]
    fn ctrl_z_undoes_a_word_and_ctrl_shift_z_redoes_it() {
        let Some(fonts) = Fonts::load() else { return };
        let theme = Theme::dark();
        let mut cx = EventCx::new(&fonts, &theme, Size::new(600.0, 400.0));
        let bounds = Rect::from_xywh(0.0, 0.0, 600.0, 400.0);

        let mut t = TextArea::new("").undo_history();
        focus::set(t.id);
        for ch in ["h", "i", "!"] {
            t.event(&mut cx, bounds, &typed(ch));
        }
        assert_eq!(t.text(), "hi!");
        assert!(t.can_undo());

        t.event(&mut cx, bounds, &ctrl(Key::Character('z')));
        assert_eq!(t.text(), "", "a run of typing undoes as one step");
        assert!(t.can_redo());

        t.event(&mut cx, bounds, &ctrl_shift(Key::Character('z')));
        assert_eq!(t.text(), "hi!");
    }

    /// Undo restores the caret, not just the text — otherwise the user has to go
    /// hunting for what changed.
    #[test]
    fn undo_restores_the_caret() {
        let Some(fonts) = Fonts::load() else { return };
        let theme = Theme::dark();
        let mut cx = EventCx::new(&fonts, &theme, Size::new(600.0, 400.0));
        let bounds = Rect::from_xywh(0.0, 0.0, 600.0, 400.0);

        let mut t = TextArea::new("ab").undo_history();
        focus::set(t.id);
        t.caret = (0, 1);
        t.event(&mut cx, bounds, &typed("X"));
        assert_eq!((t.text().as_str(), t.caret), ("aXb", (0, 2)));

        t.event(&mut cx, bounds, &ctrl(Key::Character('z')));
        assert_eq!((t.text().as_str(), t.caret), ("ab", (0, 1)));
    }

    /// With no history enabled, Ctrl+Z is inert — it must not eat the document.
    #[test]
    fn undo_is_inert_when_no_history_is_enabled() {
        let Some(fonts) = Fonts::load() else { return };
        let theme = Theme::dark();
        let mut cx = EventCx::new(&fonts, &theme, Size::new(600.0, 400.0));
        let bounds = Rect::from_xywh(0.0, 0.0, 600.0, 400.0);

        let mut t = ta("hello"); // no .undo_history()
        t.event(&mut cx, bounds, &ctrl(Key::Character('z')));
        assert_eq!(t.text(), "hello");
        assert!(!t.can_undo());
    }

    #[test]
    fn ctrl_home_and_end_go_to_the_ends_of_the_document() {
        let Some(fonts) = Fonts::load() else { return };
        let theme = Theme::dark();
        let mut cx = EventCx::new(&fonts, &theme, Size::new(600.0, 400.0));
        let bounds = Rect::from_xywh(0.0, 0.0, 600.0, 400.0);

        let mut t = ta("one\ntwo\nthree");
        t.caret = (1, 1);

        t.event(&mut cx, bounds, &ctrl(Key::End));
        assert_eq!(t.caret, (2, 5), "ctrl+End goes to the end of the document");

        t.event(&mut cx, bounds, &ctrl(Key::Home));
        assert_eq!(
            t.caret,
            (0, 0),
            "ctrl+Home goes to the start of the document"
        );

        // Bare Home/End still work on the line.
        t.caret = (1, 1);
        t.event(&mut cx, bounds, &key(Key::End));
        assert_eq!(t.caret, (1, 3));
        t.event(&mut cx, bounds, &key(Key::Home));
        assert_eq!(t.caret, (1, 0));

        // Shift+Ctrl+End selects to the end of the document.
        t.caret = (0, 0);
        t.anchor = None;
        t.event(&mut cx, bounds, &ctrl_shift(Key::End));
        assert_eq!(t.anchor, Some((0, 0)));
        assert_eq!(t.caret, (2, 5));
    }

    /// Home goes to the *code*, not to column 0 with the indentation in between —
    /// but column 0 still has to be reachable, so the two toggle.
    #[test]
    fn home_toggles_between_the_indent_and_column_zero() {
        let Some(fonts) = Fonts::load() else { return };
        let theme = Theme::dark();
        let mut cx = EventCx::new(&fonts, &theme, Size::new(600.0, 400.0));
        let bounds = Rect::from_xywh(0.0, 0.0, 600.0, 400.0);

        let mut t = ta("    let x = 1;"); // four spaces of indent
        t.caret = (0, 11);

        t.event(&mut cx, bounds, &key(Key::Home));
        assert_eq!(
            t.caret,
            (0, 4),
            "first Home lands on the code, not column 0"
        );

        t.event(&mut cx, bounds, &key(Key::Home));
        assert_eq!(t.caret, (0, 0), "already at the indent -> go to column 0");

        t.event(&mut cx, bounds, &key(Key::Home));
        assert_eq!(t.caret, (0, 4), "and back again: the two toggle");

        // A line with no indent: Home is just column 0, and stays there.
        let mut t = ta("no indent");
        t.caret = (0, 5);
        t.event(&mut cx, bounds, &key(Key::Home));
        assert_eq!(t.caret, (0, 0));
        t.event(&mut cx, bounds, &key(Key::Home));
        assert_eq!(t.caret, (0, 0), "nothing to toggle to");
    }

    #[test]
    fn ctrl_arrows_jump_by_word() {
        let Some(fonts) = Fonts::load() else { return };
        let theme = Theme::dark();
        let mut cx = EventCx::new(&fonts, &theme, Size::new(600.0, 400.0));
        let bounds = Rect::from_xywh(0.0, 0.0, 600.0, 400.0);

        let mut t = ta("let x = 1");
        t.caret = (0, 0);

        t.event(&mut cx, bounds, &ctrl(Key::Right));
        assert_eq!(t.caret, (0, 3), "end of 'let'");
        t.event(&mut cx, bounds, &ctrl(Key::Right));
        assert_eq!(t.caret, (0, 5), "skips the gap, end of 'x'");

        t.event(&mut cx, bounds, &ctrl(Key::Left));
        assert_eq!(t.caret, (0, 4), "back to the start of 'x'");
        t.event(&mut cx, bounds, &ctrl(Key::Left));
        assert_eq!(t.caret, (0, 0), "over the gap, start of 'let'");
    }

    #[test]
    fn spans_become_contiguous_runs_covering_the_line() {
        let red = Color::rgb8(255, 0, 0);
        let blue = Color::rgb8(0, 0, 255);
        let default = Color::WHITE;

        // "let x = 1"  with `let` red and `1` blue.
        let spans = [
            Span {
                start: 0,
                end: 3,
                color: red,
            },
            Span {
                start: 8,
                end: 9,
                color: blue,
            },
        ];
        let got = runs(9, &spans, default);
        assert_eq!(
            got,
            vec![(0, 3, red), (3, 8, default), (8, 9, blue)],
            "runs must tile the whole line, filling gaps with the default colour"
        );
    }

    #[test]
    fn overlapping_and_out_of_range_spans_are_ignored() {
        let red = Color::rgb8(255, 0, 0);
        let default = Color::WHITE;
        let spans = [
            Span {
                start: 0,
                end: 4,
                color: red,
            },
            Span {
                start: 2,
                end: 6,
                color: red,
            }, // overlaps the previous
            Span {
                start: 50,
                end: 60,
                color: red,
            }, // past the end
        ];
        let got = runs(5, &spans, default);
        assert_eq!(got, vec![(0, 4, red), (4, 5, default)]);
    }
}
