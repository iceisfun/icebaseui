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
    /// Hairline gap between tightly-coupled parts: an icon and its label, a row's
    /// vertical breathing room.
    pub xs: f32,
    /// Padding inside a small control, and the gap between adjacent controls in a
    /// toolbar or tab strip.
    pub sm: f32,
    /// The default padding for panel and menu content — the step to reach for when
    /// no other step is clearly right.
    pub md: f32,
    /// Horizontal padding for button labels, and the gap between distinct groups
    /// (e.g. status-bar sections).
    pub lg: f32,
    /// The largest step, for separating major regions. No widget currently uses it;
    /// it exists so apps have a top step that scales with the rest.
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
    /// Small in-place chrome: tabs, close buttons, selection and hover highlights.
    pub sm: f32,
    /// Standalone controls and floating surfaces: buttons, popup menus.
    pub md: f32,
    /// Large overlays that should read as detached from the window, such as the
    /// command palette.
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
///
/// No built-in widget animates yet, so nothing reads these today; they are here so
/// that when widgets do animate they agree on durations instead of each inventing
/// one. [`Theme::scaled`] deliberately leaves them alone — time does not scale with
/// font size.
#[derive(Clone, Copy, Debug)]
pub struct Motion {
    /// Immediate feedback the user should not perceive as an animation: hover and
    /// press state changes.
    pub fast_ms: u32,
    /// The default for a visible transition, e.g. a panel expanding or a popup
    /// fading in.
    pub normal_ms: u32,
    /// Deliberately slow, for large movements the eye needs to follow, such as a
    /// docked panel sliding across the window.
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
    /// Human-readable name, for theme pickers and settings persistence. Not an
    /// identity — nothing looks a theme up by it.
    pub name: String,
    /// Semantic colors. The only tokens a light/dark swap actually changes.
    pub palette: Palette,
    /// Padding and gap steps; scaled by [`Theme::scaled`].
    pub spacing: Spacing,
    /// Corner radii; scaled by [`Theme::scaled`].
    pub radius: Radius,
    /// Font families and sizes; sizes are scaled by [`Theme::scaled`], families are not.
    pub typography: Typography,
    /// Animation durations; deliberately *not* scaled by [`Theme::scaled`].
    pub motion: Motion,
}

impl Default for Theme {
    fn default() -> Self {
        Theme::dark()
    }
}

impl Theme {
    /// A copy of this theme with its **spacing, radii, and type sizes** scaled by
    /// `factor`, so chrome stays proportional to a scaled font size.
    ///
    /// [`App`](crate::App) derives the active theme this way from the global
    /// [text scale](crate::text::scale); colors and motion are left alone.
    pub fn scaled(&self, factor: f32) -> Theme {
        let mut theme = self.clone();
        let s = |v: f32| v * factor;
        theme.spacing = Spacing {
            xs: s(self.spacing.xs),
            sm: s(self.spacing.sm),
            md: s(self.spacing.md),
            lg: s(self.spacing.lg),
            xl: s(self.spacing.xl),
        };
        theme.radius = Radius {
            sm: s(self.radius.sm),
            md: s(self.radius.md),
            lg: s(self.radius.lg),
        };
        theme.typography.size = s(self.typography.size);
        theme.typography.size_small = s(self.typography.size_small);
        theme.typography.size_heading = s(self.typography.size_heading);
        theme
    }

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
