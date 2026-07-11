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

use std::cell::Cell;
use std::rc::Rc;

use ab_glyph::{Font, FontVec, PxScale, ScaleFont};
use baseui_core::Size;

pub use baseui_core::FontId;

/// Smallest / largest allowed global text scale.
pub const MIN_SCALE: f32 = 0.5;
pub const MAX_SCALE: f32 = 3.0;

thread_local! {
    static TEXT_SCALE: Cell<f32> = const { Cell::new(1.0) };
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

    /// Advance width of a single character (useful for monospace grids).
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
