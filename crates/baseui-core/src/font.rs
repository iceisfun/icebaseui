//! Font identity.
//!
//! [`FontId`] names *which* font a run of text uses, without knowing anything
//! about how fonts are loaded or rasterized (that lives in the `baseui` crate).
//! Keeping it here lets the dependency-free [`paint`](crate::paint) display list
//! reference fonts — including icon fonts — by identity.

/// Selects a loaded font family.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Default)]
pub enum FontId {
    /// Proportional UI font.
    #[default]
    Ui,
    /// Fixed-width font (hex editors, code, terminals).
    Mono,
    /// A registered icon font, by index. Icon fonts map glyphs onto private-use
    /// code points; see the `baseui::icon` module.
    Icon(u16),
}
