//! The paint display list.
//!
//! A [`Scene`] is a flat, backend-agnostic list of drawing commands. The widget
//! tree emits into a `Scene`; a renderer (in the `baseui` crate) consumes it and
//! turns it into GPU work. Keeping the display list here — with no rendering,
//! font, or GPU dependency — means widgets can describe *what* to draw without
//! knowing *how* it is drawn.
//!
//! Coordinates are logical (DPI-independent) pixels, origin top-left.
//!
//! ```
//! use baseui_core::{Color, Rect};
//! use baseui_core::paint::Scene;
//!
//! let mut scene = Scene::new();
//! scene.rounded_rect(Rect::from_xywh(8.0, 8.0, 120.0, 32.0), Color::WHITE, 6.0);
//! scene.text(baseui_core::Point::new(16.0, 16.0), "Hello", 14.0, Color::BLACK);
//! assert_eq!(scene.commands().len(), 2);
//! ```

use crate::{Color, FontId, Point, Rect};

/// A filled, optionally rounded and/or bordered rectangle.
#[derive(Clone, Copy, Debug)]
pub struct RectShape {
    pub rect: Rect,
    pub fill: Color,
    /// Uniform corner radius in logical pixels (`0.0` = square corners).
    pub corner_radius: f32,
    /// Border thickness in logical pixels, drawn inset from the rect edge
    /// (`0.0` = no border).
    pub border_width: f32,
    pub border_color: Color,
}

impl RectShape {
    /// A solid, square-cornered fill.
    pub fn fill(rect: Rect, color: Color) -> Self {
        RectShape {
            rect,
            fill: color,
            corner_radius: 0.0,
            border_width: 0.0,
            border_color: Color::TRANSPARENT,
        }
    }

    pub fn with_corner_radius(mut self, radius: f32) -> Self {
        self.corner_radius = radius;
        self
    }

    pub fn with_border(mut self, width: f32, color: Color) -> Self {
        self.border_width = width;
        self.border_color = color;
        self
    }
}

/// A run of text drawn at a baseline-independent top-left anchor.
///
/// Layout (advance widths, line height) and rasterization are performed by the
/// renderer; this shape only carries intent.
#[derive(Clone, Debug)]
pub struct TextShape {
    /// Top-left anchor of the text's layout box, in logical pixels.
    pub pos: Point,
    pub text: String,
    /// Font size in logical pixels.
    pub size: f32,
    pub color: Color,
    /// Which font family to render with (UI, monospace, or an icon font).
    pub font: FontId,
}

/// A single drawable primitive.
#[derive(Clone, Debug)]
pub enum Primitive {
    Rect(RectShape),
    Text(TextShape),
}

/// One entry in a scene's command stream. Clip commands bracket primitives and
/// nest; the renderer resolves the effective clip as the intersection of the
/// current clip stack.
#[derive(Clone, Debug)]
pub enum Command {
    Draw(Primitive),
    PushClip(Rect),
    PopClip,
}

/// An ordered list of drawing commands for one frame.
///
/// Cheap to build and clear; reuse one `Scene` across frames by calling
/// [`Scene::clear`] rather than allocating a new one.
#[derive(Clone, Debug, Default)]
pub struct Scene {
    commands: Vec<Command>,
    /// Commands emitted inside an overlay scope, drawn *after* (above) the main
    /// list. Used for popups, dropdown menus, and tooltips.
    overlay: Vec<Command>,
    overlay_depth: u32,
}

impl Scene {
    pub fn new() -> Self {
        Scene {
            commands: Vec::new(),
            overlay: Vec::new(),
            overlay_depth: 0,
        }
    }

    /// Remove all commands, retaining allocated capacity for reuse next frame.
    pub fn clear(&mut self) {
        self.commands.clear();
        self.overlay.clear();
        self.overlay_depth = 0;
    }

    /// The main (base-layer) command stream.
    pub fn commands(&self) -> &[Command] {
        &self.commands
    }

    /// The overlay command stream, drawn above the main layer (popups/menus).
    pub fn overlay(&self) -> &[Command] {
        &self.overlay
    }

    pub fn is_empty(&self) -> bool {
        self.commands.is_empty() && self.overlay.is_empty()
    }

    /// Enter an overlay scope: commands emitted until the matching
    /// [`Scene::end_overlay`] are drawn above the base layer, with their own
    /// clip stack (they escape enclosing clips). Scopes may nest.
    pub fn begin_overlay(&mut self) {
        self.overlay_depth += 1;
    }

    /// Leave the current overlay scope.
    pub fn end_overlay(&mut self) {
        self.overlay_depth = self.overlay_depth.saturating_sub(1);
    }

    fn emit(&mut self, command: Command) {
        if self.overlay_depth > 0 {
            self.overlay.push(command);
        } else {
            self.commands.push(command);
        }
    }

    /// Push a fully-specified rectangle shape.
    pub fn push_rect(&mut self, shape: RectShape) {
        self.emit(Command::Draw(Primitive::Rect(shape)));
    }

    /// Convenience: a solid, square-cornered fill.
    pub fn rect(&mut self, rect: Rect, color: Color) {
        self.push_rect(RectShape::fill(rect, color));
    }

    /// Convenience: a solid fill with rounded corners.
    pub fn rounded_rect(&mut self, rect: Rect, color: Color, radius: f32) {
        self.push_rect(RectShape::fill(rect, color).with_corner_radius(radius));
    }

    /// Convenience: a rounded outline (transparent fill, colored border).
    pub fn stroke_rect(&mut self, rect: Rect, color: Color, width: f32, radius: f32) {
        self.push_rect(
            RectShape::fill(rect, Color::TRANSPARENT)
                .with_corner_radius(radius)
                .with_border(width, color),
        );
    }

    /// Push a fully-specified text shape.
    pub fn push_text(&mut self, shape: TextShape) {
        self.emit(Command::Draw(Primitive::Text(shape)));
    }

    /// Convenience: UI-font text at `pos`.
    pub fn text(&mut self, pos: Point, text: impl Into<String>, size: f32, color: Color) {
        self.push_text(TextShape {
            pos,
            text: text.into(),
            size,
            color,
            font: FontId::Ui,
        });
    }

    /// Convenience: text at `pos` in a specific font (UI, monospace, or icon).
    pub fn text_font(
        &mut self,
        pos: Point,
        text: impl Into<String>,
        size: f32,
        color: Color,
        font: FontId,
    ) {
        self.push_text(TextShape {
            pos,
            text: text.into(),
            size,
            color,
            font,
        });
    }

    /// Begin a clip region; subsequent primitives are clipped to the
    /// intersection of this and any enclosing clip. Balance with [`Scene::pop_clip`].
    pub fn push_clip(&mut self, rect: Rect) {
        self.emit(Command::PushClip(rect));
    }

    /// End the most recent clip region.
    pub fn pop_clip(&mut self) {
        self.emit(Command::PopClip);
    }
}
