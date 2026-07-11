//! The BaseUI painter demo (milestone M2).
//!
//! Draws a mock application frame — menu bar, side panel, content cards, and a
//! status bar — using only the 2D painter: rounded rects, borders, clipping,
//! and text. No widget system yet; this exercises the render layer directly.
//!
//! ```text
//! cargo run -p hello
//! BASEUI_THEME=light cargo run -p hello
//! ```

use baseui::paint::RectShape;
use baseui::{App, Frame, Point, Rect, Scene, Theme};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    let theme = match std::env::var("BASEUI_THEME").as_deref() {
        Ok("light") => Theme::light(),
        _ => Theme::dark(),
    };

    App::new()
        .with_title("BaseUI — Painter Demo")
        .with_size(1000, 700)
        .with_theme(theme)
        .on_frame(draw_demo)
        .run()
}

fn draw_demo(scene: &mut Scene, frame: &Frame<'_>) {
    let p = &frame.theme.palette;
    let sp = frame.theme.spacing;
    let rad = frame.theme.radius;
    let w = frame.size.width;
    let h = frame.size.height;

    let menu_h = 34.0;
    let status_h = 26.0;
    let side_w = 220.0;

    // Menu bar.
    scene.rect(Rect::from_xywh(0.0, 0.0, w, menu_h), p.surface);
    let menu_items = ["File", "Edit", "View", "Window", "Tools", "Help"];
    let mut mx = sp.lg;
    for item in menu_items {
        scene.text(Point::new(mx, 9.0), item, 14.0, p.text);
        mx += text_width(item, 14.0) + sp.xl;
    }

    // Left side panel.
    let side = Rect::from_xywh(0.0, menu_h, side_w, h - menu_h - status_h);
    scene.rect(side, p.surface_variant);
    scene.rect(
        Rect::from_xywh(side_w - 1.0, menu_h, 1.0, side.height()),
        p.border,
    );
    let tree = [
        (0, "Scene Collection", p.text),
        (1, "Camera", p.text_muted),
        (1, "Cube", p.accent),
        (1, "Light", p.text_muted),
        (0, "Materials", p.text),
        (1, "Metal", p.text_muted),
        (1, "Glass", p.text_muted),
    ];
    let mut ty = menu_h + sp.md;
    for (depth, label, color) in tree {
        let indent = sp.lg + depth as f32 * sp.xl;
        if label == "Cube" {
            // Selection highlight row.
            scene.rounded_rect(
                Rect::from_xywh(sp.sm, ty - 2.0, side_w - sp.sm * 2.0, 22.0),
                p.selection,
                rad.sm,
            );
        }
        scene.text(Point::new(indent, ty), label, 13.0, color);
        ty += 24.0;
    }

    // Content area with two "cards".
    let content = Rect::from_xywh(
        side_w + sp.md,
        menu_h + sp.md,
        w - side_w - sp.md * 2.0,
        h - menu_h - status_h - sp.md * 2.0,
    );
    scene.push_clip(content);
    let card_w = (content.width() - sp.md) / 2.0;
    for (i, title) in ["Viewport", "Inspector"].iter().enumerate() {
        let cx = content.left() + i as f32 * (card_w + sp.md);
        let card = Rect::from_xywh(cx, content.top(), card_w, 200.0);
        scene.push_rect(
            RectShape::fill(card, p.surface)
                .with_corner_radius(rad.lg)
                .with_border(1.0, p.border),
        );
        // Card header.
        scene.text(
            Point::new(card.left() + sp.lg, card.top() + sp.lg),
            *title,
            16.0,
            p.text,
        );
        // Accent button.
        let btn = Rect::from_xywh(card.left() + sp.lg, card.top() + 48.0, 96.0, 30.0);
        scene.rounded_rect(btn, p.accent, rad.md);
        scene.text(
            Point::new(btn.left() + sp.lg, btn.top() + 7.0),
            "Apply",
            13.0,
            p.on_accent,
        );
        // Body copy.
        scene.text(
            Point::new(card.left() + sp.lg, card.top() + 96.0),
            "Rounded rects, borders,\nclipping and text — all\nfrom the 2D painter.",
            13.0,
            p.text_muted,
        );
    }
    scene.pop_clip();

    // Status bar.
    let status = Rect::from_xywh(0.0, h - status_h, w, status_h);
    scene.rect(status, p.surface);
    scene.rect(Rect::from_xywh(0.0, h - status_h, w, 1.0), p.border);
    scene.text(Point::new(sp.lg, h - status_h + 5.0), "Ready", 12.0, p.success);
    scene.text(
        Point::new(120.0, h - status_h + 5.0),
        "BaseUI M2 — painter online",
        12.0,
        p.text_muted,
    );
}

/// Rough advance-width estimate for laying out menu items (the real metric
/// lives in the renderer; this is just for demo spacing).
fn text_width(s: &str, size: f32) -> f32 {
    s.chars().count() as f32 * size * 0.55
}
