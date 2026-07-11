//! [`Label`] — static or reactive text.

use baseui_core::paint::Scene;
use baseui_core::{Color, Point, Rect, Size};

use super::{LayoutCx, PaintCx, Widget};
use crate::layout::Constraints;
use crate::text::FontId;

/// A run of text. The content may be static or a closure that reads reactive
/// state, so a label can re-render new text when a signal changes.
///
/// ```no_run
/// use baseui::widget::Label;
/// use baseui::core::create_signal;
///
/// let count = create_signal(0);
/// let _static = Label::new("Ready");
/// let _live = Label::dynamic(move || format!("Count: {}", count.get()));
/// ```
pub struct Label {
    content: Box<dyn FnMut() -> String>,
    size: f32,
    color: Option<Color>,
    font: FontId,
    /// Text captured during the last layout, reused in paint.
    cached: String,
}

impl Label {
    /// A label with fixed text.
    pub fn new(text: impl Into<String>) -> Self {
        let text = text.into();
        Label {
            content: Box::new(move || text.clone()),
            size: 14.0,
            color: None,
            font: FontId::Ui,
            cached: String::new(),
        }
    }

    /// A label whose text is recomputed each frame (typically reading a signal).
    pub fn dynamic(content: impl FnMut() -> String + 'static) -> Self {
        Label {
            content: Box::new(content),
            size: 14.0,
            color: None,
            font: FontId::Ui,
            cached: String::new(),
        }
    }

    /// Set the font size in logical pixels.
    pub fn size(mut self, size: f32) -> Self {
        self.size = size;
        self
    }

    /// Override the text color (defaults to the theme's text color).
    pub fn color(mut self, color: Color) -> Self {
        self.color = Some(color);
        self
    }

    /// Render in the monospace family.
    pub fn mono(mut self) -> Self {
        self.font = FontId::Mono;
        self
    }
}

impl Widget for Label {
    fn layout(&mut self, cx: &mut LayoutCx<'_>, constraints: Constraints) -> Size {
        self.cached = (self.content)();
        let size = cx.fonts.measure(&self.cached, self.size, self.font);
        constraints.constrain(size)
    }

    fn paint(&mut self, cx: &mut PaintCx<'_>, bounds: Rect, scene: &mut Scene) {
        let color = self.color.unwrap_or(cx.theme.palette.text);
        scene.push_text(baseui_core::paint::TextShape {
            pos: Point::new(bounds.left(), bounds.top()),
            text: self.cached.clone(),
            size: self.size,
            color,
            mono: self.font == FontId::Mono,
        });
    }
}
