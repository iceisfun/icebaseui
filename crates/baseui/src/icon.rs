//! Built-in glyph icons.
//!
//! For dogfooding, BaseUI ships a small set of icons as **font glyphs** — Unicode
//! geometric/symbol characters that ship with essentially every system UI font
//! (DejaVu, Noto, Segoe, San Francisco, …). They render through the normal text
//! path, so they inherit color and crisp anti-aliasing for free and need no
//! asset bundling.
//!
//! This is intentionally a stopgap. The SOW's feature-gated icon *packs*
//! (Material, Lucide, Codicons, Phosphor) and custom SVG sets will plug in later
//! behind an [`Icon`] abstraction once the painter grows vector-path rendering;
//! at that point these constants become the `Builtin` variant and callers that
//! use [`Icon::glyph`] keep working unchanged.

/// An icon to draw. Today it is always a single font glyph; the enum leaves room
/// for atlas-backed image/SVG icons without breaking call sites.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Icon {
    /// A single character rendered with the UI font.
    Glyph(char),
}

impl Icon {
    /// Build an icon from an arbitrary character.
    pub const fn glyph(ch: char) -> Self {
        Icon::Glyph(ch)
    }

    /// The character to render.
    pub fn ch(self) -> char {
        match self {
            Icon::Glyph(c) => c,
        }
    }
}

/// Convenience: named built-in glyph icons.
///
/// These are the characters used across the built-in widgets. Applications may
/// use any `char`; these just spare you memorizing code points.
pub mod glyphs {
    /// Filled "eye"-like disc — a visibility toggle (◉).
    pub const EYE: char = '\u{25C9}';
    /// Filled circle (●).
    pub const CIRCLE: char = '\u{25CF}';
    /// Hollow circle (○).
    pub const CIRCLE_OUTLINE: char = '\u{25CB}';
    /// Filled star (★).
    pub const STAR: char = '\u{2605}';
    /// Hollow star (☆).
    pub const STAR_OUTLINE: char = '\u{2606}';
    /// Filled diamond (◆) — e.g. a render/enable toggle.
    pub const DIAMOND: char = '\u{25C6}';
    /// Filled square (■).
    pub const SQUARE: char = '\u{25A0}';
    /// Gear / settings (⚙).
    pub const GEAR: char = '\u{2699}';
    /// Warning triangle (⚠).
    pub const WARNING: char = '\u{26A0}';
    /// Check mark (✓).
    pub const CHECK: char = '\u{2713}';
    /// Ballot X (✗).
    pub const CROSS: char = '\u{2717}';
    /// Bullet (•).
    pub const DOT: char = '\u{2022}';
    /// Right-pointing triangle — collapsed disclosure (▸).
    pub const CHEVRON_RIGHT: char = '\u{25B8}';
    /// Down-pointing triangle — expanded disclosure (▾).
    pub const CHEVRON_DOWN: char = '\u{25BE}';
    /// Wrench (🔧 is emoji; use this geometric spanner-ish gear instead) — kept as
    /// the gear for portability.
    pub const TOOL: char = GEAR;
}
