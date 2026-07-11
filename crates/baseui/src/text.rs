//! Fonts and text measurement (CPU-side, GPU-free).
//!
//! [`Fonts`] loads a default UI sans-serif and a monospace face (via `fontdb`)
//! and exposes [`measure`](Fonts::measure) / line metrics over them using
//! `ab_glyph`. It is deliberately independent of the renderer so the layout
//! system can size text without touching the GPU; the glyph atlas
//! (`render::text`) borrows the same [`Fonts`] for rasterization.
//!
//! Two font families exist today — the UI family and [`FontId::Mono`] — which is
//! what the hex editor and other fixed-width views need. See `docs/rich-text.md`
//! for how this grows into styled runs and a cached layout.

use std::rc::Rc;

use ab_glyph::{Font, FontVec, PxScale, ScaleFont};
use baseui_core::Size;

/// Selects which loaded font family to use.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Default)]
pub enum FontId {
    /// Proportional UI font.
    #[default]
    Ui,
    /// Fixed-width font (hex editors, code, terminals).
    Mono,
}

/// The loaded font faces plus measurement helpers.
pub struct Fonts {
    ui: FontVec,
    mono: FontVec,
}

impl Fonts {
    /// Load the default UI and monospace faces from the system font database.
    /// Returns `None` only if no usable face could be found at all.
    pub fn load() -> Option<Rc<Fonts>> {
        let mut db = fontdb::Database::new();
        db.load_system_fonts();

        let ui = load_family(&db, fontdb::Family::SansSerif)?;
        // Monospace is strongly preferred, but fall back to the UI face so text
        // still renders (just not perfectly aligned) on minimal systems.
        let mono = load_family(&db, fontdb::Family::Monospace)
            .or_else(|| load_family(&db, fontdb::Family::SansSerif))?;

        Some(Rc::new(Fonts { ui, mono }))
    }

    /// The `ab_glyph` face backing a family.
    pub(crate) fn face(&self, id: FontId) -> &FontVec {
        match id {
            FontId::Ui => &self.ui,
            FontId::Mono => &self.mono,
        }
    }

    /// Height of a single line at `size` logical px (ascent − descent + line gap).
    pub fn line_height(&self, size: f32, id: FontId) -> f32 {
        let scaled = self.face(id).as_scaled(PxScale::from(size));
        scaled.height() + scaled.line_gap()
    }

    /// Distance from the layout-box top to the text baseline at `size`.
    pub fn ascent(&self, size: f32, id: FontId) -> f32 {
        self.face(id).as_scaled(PxScale::from(size)).ascent()
    }

    /// Advance width of a single character (useful for monospace grids).
    pub fn char_advance(&self, ch: char, size: f32, id: FontId) -> f32 {
        let font = self.face(id);
        font.as_scaled(PxScale::from(size))
            .h_advance(font.glyph_id(ch))
    }

    /// Measure the bounding size of `text` at `size` logical px, honoring
    /// embedded newlines. Width is the widest line; height is
    /// `line_count × line_height`.
    pub fn measure(&self, text: &str, size: f32, id: FontId) -> Size {
        let font = self.face(id);
        let scaled = font.as_scaled(PxScale::from(size));
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
