//! Icons.
//!
//! An [`Icon`] is either a glyph in the UI font ([`Icon::glyph`]) or a glyph in a
//! registered **icon font** ([`Icon::font`]). Icon fonts render through the same
//! glyph atlas as text, so they inherit color and anti-aliasing for free and
//! need no vector-path rasterizer.
//!
//! Two sources ship in the box:
//! - [`glyphs`] — a handful of geometric symbols from the system UI font, always
//!   available (no assets).
//! - [`gis`] — the real **font-gis** pack (367 GIS/spatial icons), embedded and
//!   gated behind the `icons-gis` feature (on by default).
//!
//! Additional packs (Tabler, Material, Lucide, …) plug in the same way: register
//! their font bytes in [`embedded_icon_fonts`] at a new [`FontId::Icon`] index
//! and generate constants for them. (Tabler ships as SVG, so it additionally
//! needs the planned vector-path painter.)

use baseui_core::FontId;

#[cfg(feature = "icons-gis")]
pub mod gis;

/// An icon to draw: a single glyph, in either the UI font or an icon font.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Icon {
    /// A character rendered with the UI font.
    Glyph(char),
    /// A character rendered with a specific (usually icon) font.
    Font(FontId, char),
}

impl Icon {
    /// An icon from a UI-font character.
    pub const fn glyph(ch: char) -> Self {
        Icon::Glyph(ch)
    }

    /// An icon from a glyph in a specific font.
    pub const fn font(font: FontId, code: char) -> Self {
        Icon::Font(font, code)
    }

    /// The character to render.
    pub fn ch(self) -> char {
        match self {
            Icon::Glyph(c) | Icon::Font(_, c) => c,
        }
    }

    /// The font this icon is drawn with.
    pub fn font_id(self) -> FontId {
        match self {
            Icon::Glyph(_) => FontId::Ui,
            Icon::Font(f, _) => f,
        }
    }
}

/// Resolve an icon from a string spec — how config files and scripts name icons.
///
/// Accepted forms:
/// - `"gis:compass"` — a [`gis`] pack icon by name (requires `icons-gis`).
/// - `"glyph:star"` — a built-in [`glyphs`] icon by name.
/// - `"★"` — any single character, used directly as a UI-font glyph.
///
/// ```
/// use baseui::icon;
/// assert!(icon::parse("glyph:star").is_some());
/// assert!(icon::parse("★").is_some());
/// assert!(icon::parse("nope:nope").is_none());
/// ```
pub fn parse(spec: &str) -> Option<Icon> {
    if let Some(name) = spec.strip_prefix("gis:") {
        #[cfg(feature = "icons-gis")]
        return gis::by_name(name);
        #[cfg(not(feature = "icons-gis"))]
        {
            let _ = name;
            return None;
        }
    }
    if let Some(name) = spec.strip_prefix("glyph:") {
        return glyphs::by_name(name);
    }
    // A bare single character is taken as a UI-font glyph.
    let mut chars = spec.chars();
    match (chars.next(), chars.next()) {
        (Some(c), None) => Some(Icon::glyph(c)),
        _ => None,
    }
}

/// Byte blobs for the embedded icon fonts, in `FontId::Icon(n)` index order.
/// [`Fonts::load`](crate::text::Fonts::load) registers these at startup.
#[allow(clippy::vec_init_then_push)] // the push is cfg-gated; vec![] doesn't fit
pub(crate) fn embedded_icon_fonts() -> Vec<&'static [u8]> {
    #[allow(unused_mut)]
    let mut fonts: Vec<&'static [u8]> = Vec::new();
    #[cfg(feature = "icons-gis")]
    fonts.push(include_bytes!("../../assets/font-gis.ttf") as &[u8]);
    fonts
}

/// Named geometric glyph icons from the system UI font. Always available.
pub mod glyphs {
    use super::Icon;

    /// Filled "eye"-like disc — a visibility toggle (◉).
    pub const EYE: Icon = Icon::glyph('\u{25C9}');
    /// Filled circle (●).
    pub const CIRCLE: Icon = Icon::glyph('\u{25CF}');
    /// Hollow circle (○).
    pub const CIRCLE_OUTLINE: Icon = Icon::glyph('\u{25CB}');
    /// Filled star (★).
    pub const STAR: Icon = Icon::glyph('\u{2605}');
    /// Hollow star (☆).
    pub const STAR_OUTLINE: Icon = Icon::glyph('\u{2606}');
    /// Filled diamond (◆) — e.g. a render/enable toggle.
    pub const DIAMOND: Icon = Icon::glyph('\u{25C6}');
    /// Filled square (■).
    pub const SQUARE: Icon = Icon::glyph('\u{25A0}');
    /// Gear / settings (⚙).
    pub const GEAR: Icon = Icon::glyph('\u{2699}');
    /// Warning triangle (⚠).
    pub const WARNING: Icon = Icon::glyph('\u{26A0}');
    /// Check mark (✓).
    pub const CHECK: Icon = Icon::glyph('\u{2713}');
    /// Ballot X (✗).
    pub const CROSS: Icon = Icon::glyph('\u{2717}');
    /// Bullet (•).
    pub const DOT: Icon = Icon::glyph('\u{2022}');
    /// Right-pointing triangle — collapsed disclosure (▸).
    pub const CHEVRON_RIGHT: Icon = Icon::glyph('\u{25B8}');
    /// Down-pointing triangle — expanded disclosure (▾).
    pub const CHEVRON_DOWN: Icon = Icon::glyph('\u{25BE}');

    /// Resolve a built-in glyph icon by name (used by [`parse`](super::parse)).
    pub fn by_name(name: &str) -> Option<Icon> {
        Some(match name {
            "eye" => EYE,
            "circle" => CIRCLE,
            "circle-outline" => CIRCLE_OUTLINE,
            "star" => STAR,
            "star-outline" => STAR_OUTLINE,
            "diamond" => DIAMOND,
            "square" => SQUARE,
            "gear" => GEAR,
            "warning" => WARNING,
            "check" => CHECK,
            "cross" => CROSS,
            "dot" => DOT,
            "chevron-right" => CHEVRON_RIGHT,
            "chevron-down" => CHEVRON_DOWN,
            _ => return None,
        })
    }
}
