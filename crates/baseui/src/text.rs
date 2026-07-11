//! Fonts and text measurement (CPU-side, GPU-free).
//!
//! [`Fonts`] loads a default UI sans-serif, a monospace face, and any embedded
//! icon fonts (via `fontdb`/`include_bytes!`), and measures text over them with
//! `ab_glyph`. It is independent of the renderer so the layout system can size
//! text without touching the GPU; the glyph atlas (`render::glyph`) borrows the
//! same [`Fonts`] for rasterization.
//!
//! Fonts are addressed by [`FontId`] (`Ui`, `Mono`, or `Icon(n)`). Icon fonts —
//! e.g. the feature-gated `font-gis` pack — map glyphs onto private-use code
//! points; see [`crate::icon`].
//!
//! # The API, in two layers
//!
//! **Simple** — sizing a widget: [`Fonts::measure`], [`Fonts::width`],
//! [`Fonts::line_height`], [`Fonts::ascent`], [`Fonts::metrics`].
//!
//! **Advanced** — anything interactive: [`Fonts::layout_line`] returns a [`Line`],
//! the cumulative x of every character boundary. Carets ([`Line::x_of`]),
//! hit-testing ([`Line::col_at`], [`Line::char_at`]), and selection geometry
//! ([`Line::span`]) are all lookups on it. Plus [`Fonts::truncate`] and
//! [`Fonts::wrap`] for fitting text into a box.
//!
//! # The invariant everything rests on
//!
//! [`Fonts::char_advance`] is the **single definition of how far the pen moves**:
//! the renderer steps by exactly it. So anything you compute by summing advances
//! lands precisely on the drawn glyphs, at any DPI and any text scale. Do not
//! re-derive advances from a rasterized pixel size — that rounds, and the error
//! accumulates along the line, so the caret drifts further from the text the
//! longer the line gets.
//!
//! All sizes and results are **logical pixels** with the [global text
//! scale](scale) already applied; you never multiply by it yourself.
//!
//! See `docs/text.md` for the guide.

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use ab_glyph::{Font, FontVec, PxScale, ScaleFont};
use baseui_core::Size;

pub use baseui_core::FontId;

/// Smallest / largest allowed global text scale.
pub const MIN_SCALE: f32 = 0.5;
pub const MAX_SCALE: f32 = 3.0;

thread_local! {
    static TEXT_SCALE: Cell<f32> = const { Cell::new(1.0) };
    static FONTS: RefCell<Option<Rc<Fonts>>> = const { RefCell::new(None) };
}

/// Publish the loaded [`Fonts`] as the process-wide handle. [`App`](crate::App)
/// calls this at startup; you only need it if you drive the widget tree yourself.
pub fn install(fonts: Rc<Fonts>) {
    FONTS.with(|f| *f.borrow_mut() = Some(fonts));
}

/// The loaded [`Fonts`], for code that has no `cx` to hand — scripts, command
/// handlers, background layout. Returns `None` before [`App`](crate::App) starts.
///
/// Inside a widget pass, prefer `cx.fonts`: it is the same object, without the
/// lookup.
pub fn fonts() -> Option<Rc<Fonts>> {
    FONTS.with(|f| f.borrow().clone())
}

/// Set the **global text scale** — a multiplier applied to every font size, in
/// both measurement ([`Fonts`]) and rasterization (the glyph atlas).
///
/// Because widgets size themselves from measured text, scaling here also grows
/// rows, buttons, fields, headers, and the hex grid; the theme's spacing and
/// radii are scaled to match by [`Theme::scaled`](crate::theme::Theme::scaled),
/// which [`App`](crate::App) applies for you. Clamped to
/// [`MIN_SCALE`]..=[`MAX_SCALE`].
pub fn set_scale(scale: f32) {
    TEXT_SCALE.with(|s| s.set(scale.clamp(MIN_SCALE, MAX_SCALE)));
    // This is global state, not a signal, so nothing else would repaint. Every
    // window's layout depends on it, so dirty them all.
    crate::window::mark_dirty();
}

/// The current global text scale (`1.0` = 100%).
pub fn scale() -> f32 {
    TEXT_SCALE.with(|s| s.get())
}

/// The loaded font faces plus measurement helpers.
pub struct Fonts {
    ui: FontVec,
    mono: FontVec,
    /// Icon fonts indexed by `FontId::Icon(n)`.
    icons: Vec<FontVec>,
}

impl Fonts {
    /// Load the default UI and monospace faces from the system font database,
    /// plus any embedded icon fonts. Returns `None` only if no usable text face
    /// could be found.
    pub fn load() -> Option<Rc<Fonts>> {
        let mut db = fontdb::Database::new();
        db.load_system_fonts();

        let ui = load_family(&db, fontdb::Family::SansSerif)?;
        let mono = load_family(&db, fontdb::Family::Monospace)
            .or_else(|| load_family(&db, fontdb::Family::SansSerif))?;

        let mut icons = Vec::new();
        for bytes in crate::icon::embedded_icon_fonts() {
            if let Ok(font) = FontVec::try_from_vec(bytes.to_vec()) {
                icons.push(font);
            }
        }

        Some(Rc::new(Fonts { ui, mono, icons }))
    }

    /// The `ab_glyph` face for a font id, if loaded. `Icon(n)` returns `None`
    /// when that icon font is not registered.
    pub(crate) fn face(&self, id: FontId) -> Option<&FontVec> {
        match id {
            FontId::Ui => Some(&self.ui),
            FontId::Mono => Some(&self.mono),
            FontId::Icon(n) => self.icons.get(n as usize),
        }
    }

    /// A face guaranteed to exist, for metrics fallback (UI font).
    fn face_or_ui(&self, id: FontId) -> &FontVec {
        self.face(id).unwrap_or(&self.ui)
    }

    /// Height of a single line at `size` logical px (ascent − descent + line gap).
    /// `size` is multiplied by the [global text scale](scale).
    pub fn line_height(&self, size: f32, id: FontId) -> f32 {
        let scaled = self.face_or_ui(id).as_scaled(PxScale::from(size * scale()));
        scaled.height() + scaled.line_gap()
    }

    /// Distance from the layout-box top to the text baseline at `size`.
    pub fn ascent(&self, size: f32, id: FontId) -> f32 {
        self.face_or_ui(id)
            .as_scaled(PxScale::from(size * scale()))
            .ascent()
    }

    /// Advance width of a single character, in logical pixels.
    ///
    /// **This is the single definition of how far the pen moves.** The renderer
    /// steps its pen by exactly this (see `render::glyph::push_text`), so any
    /// caret, hit-test, or column layout built by summing it lands precisely on
    /// the drawn glyphs — at any DPI, at any text scale. Do not re-derive
    /// advances from a rasterized pixel size; that rounds, and the error
    /// accumulates along the line.
    pub fn char_advance(&self, ch: char, size: f32, id: FontId) -> f32 {
        let Some(font) = self.face(id) else {
            return 0.0;
        };
        font.as_scaled(PxScale::from(size * scale()))
            .h_advance(font.glyph_id(ch))
    }

    /// Measure the bounding size of `text` at `size` logical px, honoring
    /// embedded newlines. Missing fonts measure as zero width.
    pub fn measure(&self, text: &str, size: f32, id: FontId) -> Size {
        let Some(font) = self.face(id) else {
            return Size::ZERO;
        };
        let scaled = font.as_scaled(PxScale::from(size * scale()));
        let line_h = scaled.height() + scaled.line_gap();

        let mut max_w = 0.0f32;
        let mut cur_w = 0.0f32;
        let mut lines = 1usize;
        for ch in text.chars() {
            if ch == '\n' {
                max_w = max_w.max(cur_w);
                cur_w = 0.0;
                lines += 1;
                continue;
            }
            cur_w += scaled.h_advance(font.glyph_id(ch));
        }
        max_w = max_w.max(cur_w);

        Size::new(max_w, line_h * lines as f32)
    }

    /// Width of a **single** line, in logical pixels. (`measure().width` for text
    /// you know has no newlines, without building a [`Size`].)
    pub fn width(&self, text: &str, size: f32, id: FontId) -> f32 {
        let Some(font) = self.face(id) else {
            return 0.0;
        };
        let scaled = font.as_scaled(PxScale::from(size * scale()));
        text.chars()
            .map(|ch| scaled.h_advance(font.glyph_id(ch)))
            .sum()
    }

    /// The full vertical metrics of one line — everything needed to place a
    /// baseline, underline, or a box around text.
    pub fn metrics(&self, size: f32, id: FontId) -> LineMetrics {
        let scaled = self.face_or_ui(id).as_scaled(PxScale::from(size * scale()));
        LineMetrics {
            ascent: scaled.ascent(),
            // ab_glyph reports descent below the baseline as negative; flip it,
            // because "how far below the baseline" is the useful quantity and a
            // sign error here is an easy way to draw an underline in the wrong
            // place.
            descent: -scaled.descent(),
            line_gap: scaled.line_gap(),
            height: scaled.height() + scaled.line_gap(),
        }
    }

    /// Lay out one line into a [`Line`] — cumulative x offsets for every
    /// character boundary.
    ///
    /// **This is the tool for carets, hit-testing, selection, and colored runs.**
    /// Build one per line you are drawing, then ask it questions; do not re-sum
    /// advances by hand. `text` should not contain newlines (they measure as
    /// zero-width and would not break).
    ///
    /// ```no_run
    /// # use baseui::text::{Fonts, FontId};
    /// # fn demo(fonts: &Fonts, click_x: f32) {
    /// let line = fonts.layout_line("fn main() {", 13.0, FontId::Mono);
    ///
    /// let caret_col = line.col_at(click_x);   // where a click lands
    /// let caret_x = line.x_of(caret_col);     // where to draw the caret
    /// let (x0, x1) = line.span(0, 2);         // the extent of "fn", for a highlight
    /// # }
    /// ```
    pub fn layout_line(&self, text: &str, size: f32, id: FontId) -> Line {
        let Some(font) = self.face(id) else {
            return Line {
                offsets: vec![0.0; text.chars().count() + 1],
            };
        };
        let scaled = font.as_scaled(PxScale::from(size * scale()));

        let mut offsets = Vec::with_capacity(text.chars().count() + 1);
        let mut x = 0.0;
        offsets.push(0.0);
        for ch in text.chars() {
            x += scaled.h_advance(font.glyph_id(ch));
            offsets.push(x);
        }
        Line { offsets }
    }

    /// Shorten `text` to fit `max_width`, appending an ellipsis if it had to cut.
    ///
    /// Returns the original string when it already fits. Cuts on character
    /// boundaries (not words) — this is for labels, tabs, and table cells that
    /// must not overflow their box.
    pub fn truncate(&self, text: &str, size: f32, id: FontId, max_width: f32) -> String {
        if self.width(text, size, id) <= max_width {
            return text.to_string();
        }
        let ellipsis = '…';
        let ell_w = self.char_advance(ellipsis, size, id);
        let budget = max_width - ell_w;
        if budget <= 0.0 {
            return String::new();
        }

        let mut out = String::new();
        let mut x = 0.0;
        for ch in text.chars() {
            let adv = self.char_advance(ch, size, id);
            if x + adv > budget {
                break;
            }
            x += adv;
            out.push(ch);
        }
        out.push(ellipsis);
        out
    }

    /// Greedy word-wrap `text` into lines no wider than `max_width`.
    ///
    /// Breaks on whitespace; a single word longer than `max_width` is broken at
    /// the character that overflows. Existing newlines are honored as hard breaks.
    ///
    /// This is *layout*, not shaping: it does not reorder, kern across breaks, or
    /// support per-span fonts. For a paragraph of styled, wrapped rich text see
    /// the galley engine planned in `docs/rich-text.md`. For plain wrapped
    /// labels and tooltips, this is the whole job.
    pub fn wrap(&self, text: &str, size: f32, id: FontId, max_width: f32) -> Vec<String> {
        let mut out = Vec::new();

        for hard_line in text.split('\n') {
            let mut current = String::new();
            let mut current_w = 0.0f32;

            for word in hard_line.split_inclusive(char::is_whitespace) {
                let word_w = self.width(word, size, id);

                // Fits on the current line: take it.
                if current.is_empty() || current_w + word_w <= max_width {
                    // A single word too long for an empty line: hard-break it.
                    if current.is_empty() && word_w > max_width {
                        let mut chunk = String::new();
                        let mut chunk_w = 0.0;
                        for ch in word.chars() {
                            let adv = self.char_advance(ch, size, id);
                            if chunk_w + adv > max_width && !chunk.is_empty() {
                                out.push(std::mem::take(&mut chunk));
                                chunk_w = 0.0;
                            }
                            chunk.push(ch);
                            chunk_w += adv;
                        }
                        current = chunk;
                        current_w = chunk_w;
                        continue;
                    }
                    current.push_str(word);
                    current_w += word_w;
                    continue;
                }

                // Does not fit: break, and start the next line with this word.
                out.push(current.trim_end().to_string());
                current = word.to_string();
                current_w = word_w;
            }

            out.push(current.trim_end().to_string());
        }

        out
    }
}

/// The vertical metrics of one line of text, in logical pixels.
///
/// ```text
///          ┌─────────────────────────  top of the line box
///  ascent  │   ██  ██
///          │   ██████   ← the baseline is `ascent` below the top
///  descent │     ██
///          │
/// line_gap │
///          └─────────────────────────  top of the *next* line, `height` below
/// ```
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LineMetrics {
    /// Distance from the top of the line box down to the baseline.
    pub ascent: f32,
    /// Distance from the baseline down to the lowest descender. **Positive.**
    pub descent: f32,
    /// Extra leading the face asks for between lines.
    pub line_gap: f32,
    /// Total advance from one line's top to the next: `ascent + descent + line_gap`.
    pub height: f32,
}

/// One laid-out line: the cumulative x offset of every character boundary.
///
/// `offsets[i]` is the x of the boundary *before* character `i`, so a line of `n`
/// characters has `n + 1` offsets and `offsets[n]` is its width. Every question a
/// text widget asks — where is the caret, which character did the user click,
/// how wide is this selection — is a lookup in here.
///
/// Offsets come from [`Fonts::char_advance`], which is the same call the renderer
/// steps its pen by, so these positions land exactly on the drawn glyphs.
#[derive(Clone, Debug, Default)]
pub struct Line {
    offsets: Vec<f32>,
}

impl Line {
    /// Number of characters (not bytes) in the line.
    pub fn len(&self) -> usize {
        self.offsets.len() - 1
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Total width of the line.
    pub fn width(&self) -> f32 {
        *self.offsets.last().unwrap_or(&0.0)
    }

    /// X of the boundary before character `col` — **where to draw a caret**.
    /// Clamped, so `x_of(len())` is the end of the line and out-of-range is safe.
    pub fn x_of(&self, col: usize) -> f32 {
        self.offsets[col.min(self.len())]
    }

    /// The character boundary **nearest** to `x` — where a click puts the caret.
    ///
    /// Nearest, not containing: clicking the right half of a glyph places the
    /// caret *after* it, which is what every text field does.
    pub fn col_at(&self, x: f32) -> usize {
        for col in 0..self.len() {
            let mid = (self.offsets[col] + self.offsets[col + 1]) * 0.5;
            if x < mid {
                return col;
            }
        }
        self.len()
    }

    /// The character actually **under** `x`, or `None` past either end — for
    /// hover, tooltips, and per-character hit targets (not carets).
    pub fn char_at(&self, x: f32) -> Option<usize> {
        if x < 0.0 || x >= self.width() {
            return None;
        }
        (0..self.len()).find(|&col| x < self.offsets[col + 1])
    }

    /// The x extent `(start_x, end_x)` of characters `start..end` — the geometry
    /// of a selection rect, a colored run's background, or a squiggle.
    pub fn span(&self, start: usize, end: usize) -> (f32, f32) {
        (self.x_of(start), self.x_of(end.max(start)))
    }

    /// The raw boundary offsets (`len() + 1` of them).
    pub fn offsets(&self) -> &[f32] {
        &self.offsets
    }
}

fn load_family(db: &fontdb::Database, family: fontdb::Family<'_>) -> Option<FontVec> {
    let query = fontdb::Query {
        families: &[family],
        ..Default::default()
    };
    let id = db
        .query(&query)
        .or_else(|| db.faces().next().map(|f| f.id))?;
    db.with_face_data(id, |data, index| {
        FontVec::try_from_vec_and_index(data.to_vec(), index).ok()
    })
    .flatten()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The contract every caret, hit-test, and column layout in the codebase
    /// leans on: a run's width is the *sum of its characters' advances*.
    ///
    /// Widgets place a caret by summing `char_advance` up to the caret column;
    /// the renderer steps its pen by the same call. If these two could disagree,
    /// the caret would drift further from the text the longer the line got —
    /// which is exactly the bug that motivated this test.
    #[test]
    fn measure_is_the_sum_of_char_advances() {
        let Some(fonts) = Fonts::load() else { return };

        for id in [FontId::Ui, FontId::Mono] {
            for size in [11.0, 13.0, 14.0, 17.0] {
                let text = "Lines do not wrap — that is what keeps the caret a prefix sum.";
                let summed: f32 = text
                    .chars()
                    .map(|ch| fonts.char_advance(ch, size, id))
                    .sum();
                let measured = fonts.measure(text, size, id).width;
                assert!(
                    (summed - measured).abs() < 0.01,
                    "{id:?} at {size}: summed advances {summed} != measured width {measured}"
                );
            }
        }
    }

    #[test]
    fn line_offsets_answer_caret_hit_test_and_span() {
        let Some(fonts) = Fonts::load() else { return };
        let line = fonts.layout_line("hello", 14.0, FontId::Ui);

        assert_eq!(line.len(), 5);
        assert_eq!(line.offsets().len(), 6, "n chars => n+1 boundaries");
        assert_eq!(line.x_of(0), 0.0);
        assert!((line.x_of(5) - line.width()).abs() < 0.001);

        // Out of range is clamped, not a panic: widgets index this with a caret
        // that may briefly outrun a shortened string.
        assert_eq!(line.x_of(99), line.width());

        // A click just past a glyph's midpoint puts the caret after it.
        let mid_of_first = line.x_of(1) * 0.5;
        assert_eq!(line.col_at(mid_of_first - 0.5), 0);
        assert_eq!(line.col_at(mid_of_first + 0.5), 1);

        // Far right lands at the end (that is where a click past the text goes).
        assert_eq!(line.col_at(10_000.0), 5);

        // char_at is *containment*, not nearest — it has no answer past the end.
        assert_eq!(line.char_at(mid_of_first), Some(0));
        assert_eq!(line.char_at(line.width() + 1.0), None);
        assert_eq!(line.char_at(-1.0), None);

        // A span is the geometry of a selection / squiggle / run background.
        let (x0, x1) = line.span(1, 3);
        assert_eq!(x0, line.x_of(1));
        assert_eq!(x1, line.x_of(3));
    }

    #[test]
    fn line_agrees_with_measure_and_with_char_advance() {
        let Some(fonts) = Fonts::load() else { return };
        let text = "The caret must land on the glyph.";
        let line = fonts.layout_line(text, 13.0, FontId::Mono);

        // The whole point: one definition of advance, three ways in.
        assert!((line.width() - fonts.width(text, 13.0, FontId::Mono)).abs() < 0.01);
        assert!((line.width() - fonts.measure(text, 13.0, FontId::Mono).width).abs() < 0.01);

        let summed: f32 = text
            .chars()
            .take(7)
            .map(|c| fonts.char_advance(c, 13.0, FontId::Mono))
            .sum();
        assert!((line.x_of(7) - summed).abs() < 0.01);
    }

    #[test]
    fn metrics_report_descent_as_a_positive_depth() {
        let Some(fonts) = Fonts::load() else { return };
        let m = fonts.metrics(14.0, FontId::Ui);

        assert!(m.ascent > 0.0);
        assert!(
            m.descent > 0.0,
            "descent is a depth below the baseline, not a signed offset"
        );
        assert!(m.line_gap >= 0.0);
        assert!((m.height - (m.ascent + m.descent + m.line_gap)).abs() < 0.01);
        assert!((m.height - fonts.line_height(14.0, FontId::Ui)).abs() < 0.01);
        assert!((m.ascent - fonts.ascent(14.0, FontId::Ui)).abs() < 0.01);
    }

    #[test]
    fn truncate_fits_within_the_budget_and_marks_the_cut() {
        let Some(fonts) = Fonts::load() else { return };
        let text = "a long label that will not fit in a narrow column";
        let max = 80.0;

        let cut = fonts.truncate(text, 12.0, FontId::Ui, max);
        assert!(
            cut.ends_with('…'),
            "a truncated string must show that it was cut"
        );
        assert!(
            fonts.width(&cut, 12.0, FontId::Ui) <= max,
            "the result, ellipsis included, must fit the budget"
        );

        // Text that already fits is returned untouched — no gratuitous ellipsis.
        let short = "ok";
        assert_eq!(fonts.truncate(short, 12.0, FontId::Ui, max), short);
    }

    #[test]
    fn wrap_breaks_on_words_and_honors_hard_newlines() {
        let Some(fonts) = Fonts::load() else { return };
        let max = 120.0;

        let lines = fonts.wrap(
            "the quick brown fox jumps over the lazy dog",
            12.0,
            FontId::Ui,
            max,
        );
        assert!(lines.len() > 1, "long text must wrap");
        for line in &lines {
            assert!(
                fonts.width(line, 12.0, FontId::Ui) <= max + 0.01,
                "wrapped line {line:?} overflows"
            );
        }
        // Wrapping must not lose or reorder words.
        assert_eq!(
            lines.join(" ").split_whitespace().collect::<Vec<_>>(),
            "the quick brown fox jumps over the lazy dog"
                .split_whitespace()
                .collect::<Vec<_>>()
        );

        // Hard newlines are breaks the wrapper must respect.
        let hard = fonts.wrap("a\nb", 12.0, FontId::Ui, 1000.0);
        assert_eq!(hard, vec!["a", "b"]);
    }

    #[test]
    fn a_word_longer_than_the_line_is_broken_rather_than_overflowing() {
        let Some(fonts) = Fonts::load() else { return };
        let max = 40.0;
        let lines = fonts.wrap("supercalifragilisticexpialidocious", 12.0, FontId::Ui, max);

        assert!(lines.len() > 1);
        for line in &lines {
            assert!(fonts.width(line, 12.0, FontId::Ui) <= max + 0.01);
        }
        assert_eq!(lines.concat(), "supercalifragilisticexpialidocious");
    }

    /// Advances are **logical**: they must not be quantized to whole physical
    /// pixels. A renderer that derived them from a rounded raster size would
    /// scale them by `round(size × dpi) / (size × dpi)` — a few percent, but it
    /// accumulates, so a long line ends up visibly out of step with its caret.
    ///
    /// Pinned by checking that advances are exactly linear in size: rounding
    /// anywhere in the chain would break that.
    #[test]
    fn advances_are_linear_in_size_not_pixel_quantized() {
        let Some(fonts) = Fonts::load() else { return };

        let a = fonts.char_advance('m', 14.0, FontId::Ui);
        let b = fonts.char_advance('m', 28.0, FontId::Ui);
        assert!(
            (b - a * 2.0).abs() < 0.01,
            "advance at 28px ({b}) should be exactly double that at 14px ({a})"
        );

        // 17.5px is what a 14px font wants on a 1.25x display: the size the old
        // renderer rounded to 18 before taking its advances.
        let frac = fonts.char_advance('m', 17.5, FontId::Ui);
        let whole = fonts.char_advance('m', 18.0, FontId::Ui);
        assert!(
            frac < whole,
            "a fractional size must produce a fractional advance, not a rounded one"
        );
    }
}
