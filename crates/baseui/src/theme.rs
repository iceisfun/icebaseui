//! The theme engine.
//!
//! A [`Theme`] is a plain, cloneable bundle of design tokens — colors, spacing,
//! typography metrics, corner radii, and animation timings. Widgets read tokens
//! from the active theme rather than hard-coding appearance, which is what makes
//! BaseUI re-skinnable and plugin-friendly.
//!
//! This foundation ships two built-in themes ([`Theme::dark`] and
//! [`Theme::light`]); applications may construct their own or tweak a built-in
//! one field by field.

use baseui_core::Color;

/// Semantic color roles used across the framework. Widgets reference these
/// roles rather than literal colors so a single theme swap restyles everything.
#[derive(Clone, Debug)]
pub struct Palette {
    /// Window / root background.
    pub background: Color,
    /// Background of raised surfaces (panels, cards, menus).
    pub surface: Color,
    /// Background of the surface behind `surface` (e.g. panel gutters).
    pub surface_variant: Color,
    /// Primary text color.
    pub text: Color,
    /// De-emphasized text (hints, secondary labels).
    pub text_muted: Color,
    /// Accent / primary interactive color.
    pub accent: Color,
    /// Text/icon color drawn on top of `accent`.
    pub on_accent: Color,
    /// Borders and separators.
    pub border: Color,
    /// Background of a hovered interactive element.
    pub hover: Color,
    /// Background of a pressed/active interactive element.
    pub active: Color,
    /// Selection highlight background.
    pub selection: Color,
    /// Error / danger color.
    pub error: Color,
    /// Warning color.
    pub warning: Color,
    /// Success / ok color.
    pub success: Color,
}

/// Spacing scale in logical pixels. Widgets compose layouts from these steps
/// instead of magic numbers so density stays consistent and themeable.
#[derive(Clone, Copy, Debug)]
pub struct Spacing {
    pub xs: f32,
    pub sm: f32,
    pub md: f32,
    pub lg: f32,
    pub xl: f32,
}

impl Default for Spacing {
    fn default() -> Self {
        Spacing {
            xs: 2.0,
            sm: 4.0,
            md: 8.0,
            lg: 12.0,
            xl: 16.0,
        }
    }
}

/// Corner radii in logical pixels.
#[derive(Clone, Copy, Debug)]
pub struct Radius {
    pub sm: f32,
    pub md: f32,
    pub lg: f32,
}

impl Default for Radius {
    fn default() -> Self {
        Radius {
            sm: 3.0,
            md: 6.0,
            lg: 10.0,
        }
    }
}

/// Typography metrics. Font *loading* is handled by the renderer; these are the
/// size/spacing tokens the layout system reasons about.
#[derive(Clone, Debug)]
pub struct Typography {
    /// Default UI font family name (resolved by the text backend).
    pub family: String,
    /// Monospace font family name.
    pub mono_family: String,
    /// Base font size in logical pixels.
    pub size: f32,
    /// Small font size (captions, badges).
    pub size_small: f32,
    /// Heading font size.
    pub size_heading: f32,
    /// Line height as a multiple of font size.
    pub line_height: f32,
}

impl Default for Typography {
    fn default() -> Self {
        Typography {
            family: "sans-serif".to_string(),
            mono_family: "monospace".to_string(),
            size: 14.0,
            size_small: 12.0,
            size_heading: 18.0,
            line_height: 1.4,
        }
    }
}

/// Animation timing tokens in milliseconds.
#[derive(Clone, Copy, Debug)]
pub struct Motion {
    pub fast_ms: u32,
    pub normal_ms: u32,
    pub slow_ms: u32,
}

impl Default for Motion {
    fn default() -> Self {
        Motion {
            fast_ms: 90,
            normal_ms: 160,
            slow_ms: 280,
        }
    }
}

/// A complete theme: the bundle of design tokens the whole framework reads from.
#[derive(Clone, Debug)]
pub struct Theme {
    pub name: String,
    pub palette: Palette,
    pub spacing: Spacing,
    pub radius: Radius,
    pub typography: Typography,
    pub motion: Motion,
}

impl Default for Theme {
    fn default() -> Self {
        Theme::dark()
    }
}

impl Theme {
    /// The default dark theme.
    pub fn dark() -> Self {
        Theme {
            name: "BaseUI Dark".to_string(),
            palette: Palette {
                background: Color::rgb8(0x1e, 0x1e, 0x22),
                surface: Color::rgb8(0x26, 0x26, 0x2b),
                surface_variant: Color::rgb8(0x2f, 0x2f, 0x36),
                text: Color::rgb8(0xe6, 0xe6, 0xea),
                text_muted: Color::rgb8(0x9a, 0x9a, 0xa4),
                accent: Color::rgb8(0x4d, 0x9c, 0xf5),
                on_accent: Color::rgb8(0xff, 0xff, 0xff),
                border: Color::rgb8(0x3a, 0x3a, 0x42),
                hover: Color::rgb8(0x33, 0x33, 0x3b),
                active: Color::rgb8(0x3d, 0x3d, 0x47),
                selection: Color::rgba8(0x4d, 0x9c, 0xf5, 0x55),
                error: Color::rgb8(0xe5, 0x5c, 0x5c),
                warning: Color::rgb8(0xe0, 0xa4, 0x4e),
                success: Color::rgb8(0x5c, 0xc9, 0x7a),
            },
            spacing: Spacing::default(),
            radius: Radius::default(),
            typography: Typography::default(),
            motion: Motion::default(),
        }
    }

    /// The default light theme.
    pub fn light() -> Self {
        Theme {
            name: "BaseUI Light".to_string(),
            palette: Palette {
                background: Color::rgb8(0xf4, 0xf4, 0xf6),
                surface: Color::rgb8(0xff, 0xff, 0xff),
                surface_variant: Color::rgb8(0xea, 0xea, 0xee),
                text: Color::rgb8(0x1c, 0x1c, 0x20),
                text_muted: Color::rgb8(0x6a, 0x6a, 0x74),
                accent: Color::rgb8(0x1a, 0x73, 0xe8),
                on_accent: Color::rgb8(0xff, 0xff, 0xff),
                border: Color::rgb8(0xd4, 0xd4, 0xda),
                hover: Color::rgb8(0xec, 0xec, 0xf1),
                active: Color::rgb8(0xdf, 0xdf, 0xe6),
                selection: Color::rgba8(0x1a, 0x73, 0xe8, 0x33),
                error: Color::rgb8(0xd3, 0x3b, 0x3b),
                warning: Color::rgb8(0xb9, 0x7a, 0x1e),
                success: Color::rgb8(0x2f, 0x9e, 0x50),
            },
            spacing: Spacing::default(),
            radius: Radius::default(),
            typography: Typography::default(),
            motion: Motion::default(),
        }
    }
}
