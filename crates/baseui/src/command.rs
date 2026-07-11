//! The command system, shortcut bindings, and the built-in Command Palette.
//!
//! Applications register **commands** — each with an id, human title, category,
//! and optional icon/color/shortcut — and a handler. Menus, toolbars, keyboard
//! shortcuts, and the [`CommandPalette`] all invoke commands by id, so behavior
//! is defined once and reachable many ways (SOW: "Commands are independent from
//! widgets").
//!
//! The registry and shortcut table live in thread-local storage (single UI
//! thread), so any code can [`register`], [`run`], or [`bind_shortcut`] without
//! threading a context object through the widget tree.
//!
//! The **Command Palette** (opened with `F1` by default) is a searchable
//! overlay listing every registered command.

use std::cell::RefCell;
use std::collections::HashMap;

use baseui_core::Color;
use baseui_core::paint::{RectShape, Scene};
use baseui_core::{Point, Rect, Size};

use crate::event::{Key, Modifiers};
use crate::icon::Icon;
use crate::text::{FontId, Fonts};
use crate::theme::Theme;

/// Metadata describing a registered command (everything except its handler).
#[derive(Clone)]
pub struct CommandMeta {
    /// Stable identifier everything else invokes the command by — menus,
    /// toolbars, shortcuts, and the palette. Registering it twice replaces it.
    pub id: String,
    /// Human-readable label, shown wherever the command is listed.
    pub title: String,
    /// Groups the command in the palette, and is matched by palette search.
    /// Defaults to `"General"`.
    pub category: String,
    /// Glyph drawn beside the title in menus, toolbars, and the palette.
    pub icon: Option<Icon>,
    /// Tint for the icon; falls back to the theme's text color.
    pub color: Option<Color>,
    /// Human-readable shortcut hint (e.g. `"Ctrl+S"`), shown in menus/palette.
    pub shortcut: Option<String>,
    /// Optional **context**. `None` = global (available everywhere). Otherwise
    /// the command is only listed in the palette — and its shortcut only fires —
    /// while a window declaring that context is focused. This is how a detached
    /// panel window offers its own commands while the main window offers all of
    /// them; see [`WindowSpec::context`](crate::window::WindowSpec::context).
    pub context: Option<String>,
}

impl CommandMeta {
    /// A global command in the `"General"` category, with no icon, color, or
    /// shortcut; refine it with the builders below.
    pub fn new(id: impl Into<String>, title: impl Into<String>) -> Self {
        CommandMeta {
            id: id.into(),
            title: title.into(),
            category: "General".to_string(),
            icon: None,
            color: None,
            shortcut: None,
            context: None,
        }
    }

    /// Restrict this command to a window context (see [`CommandMeta::context`]).
    pub fn context(mut self, context: impl Into<String>) -> Self {
        self.context = Some(context.into());
        self
    }

    /// Group the command under `category` in the palette.
    pub fn category(mut self, category: impl Into<String>) -> Self {
        self.category = category.into();
        self
    }

    /// Show `icon` beside the title wherever the command is listed.
    pub fn icon(mut self, icon: Icon) -> Self {
        self.icon = Some(icon);
        self
    }

    /// Tint the icon; without one it inherits the theme's text color.
    pub fn color(mut self, color: Color) -> Self {
        self.color = Some(color);
        self
    }

    /// Advertise a chord (e.g. `"Ctrl+S"`). [`register`] also installs it in the
    /// shortcut table, so this both documents *and* binds the key.
    pub fn shortcut(mut self, shortcut: impl Into<String>) -> Self {
        self.shortcut = Some(shortcut.into());
        self
    }
}

type Handler = Box<dyn FnMut()>;

struct Entry {
    meta: CommandMeta,
    handler: Option<Handler>,
}

#[derive(Default)]
struct Registry {
    entries: Vec<Entry>,
    /// Chord string (e.g. `"Ctrl+S"`) → command id.
    shortcuts: HashMap<String, String>,
}

thread_local! {
    static REGISTRY: RefCell<Registry> = RefCell::new(Registry::default());
    /// Context of the focused window; scopes which commands are visible/bound.
    static ACTIVE_CONTEXT: RefCell<Option<String>> = const { RefCell::new(None) };
}

/// Set the active command context (the App does this when window focus changes).
pub fn set_active_context(context: Option<String>) {
    ACTIVE_CONTEXT.with(|c| *c.borrow_mut() = context);
}

/// The active command context.
pub fn active_context() -> Option<String> {
    ACTIVE_CONTEXT.with(|c| c.borrow().clone())
}

/// Whether a command with this context is reachable right now.
fn in_scope(context: &Option<String>) -> bool {
    match context {
        None => true, // global
        Some(c) => active_context().as_deref() == Some(c.as_str()),
    }
}

/// Register a command and its handler. Re-registering the same id replaces it.
pub fn register(meta: CommandMeta, handler: impl FnMut() + 'static) {
    REGISTRY.with(|r| {
        let mut r = r.borrow_mut();
        if let Some(chord) = meta.shortcut.clone() {
            r.shortcuts.insert(normalize_chord(&chord), meta.id.clone());
        }
        let entry = Entry {
            meta,
            handler: Some(Box::new(handler)),
        };
        if let Some(existing) = r.entries.iter_mut().find(|e| e.meta.id == entry.meta.id) {
            *existing = entry;
        } else {
            r.entries.push(entry);
        }
    });
}

/// Run the command with `id`, if registered. The handler is taken out of the
/// registry while it runs, so a command may safely trigger others.
pub fn run(id: &str) {
    let handler = REGISTRY.with(|r| {
        r.borrow_mut()
            .entries
            .iter_mut()
            .find(|e| e.meta.id == id)
            .and_then(|e| e.handler.take())
    });
    if let Some(mut handler) = handler {
        handler();
        REGISTRY.with(|r| {
            if let Some(e) = r.borrow_mut().entries.iter_mut().find(|e| e.meta.id == id) {
                e.handler = Some(handler);
            }
        });
        // A command may change anything — including global state that isn't a
        // signal (text scale, theme, dock layout). Repaint every window rather
        // than only the one that happened to dispatch it.
        crate::window::mark_dirty();
    }
}

/// Every registered command's metadata, in registration order.
pub fn all() -> Vec<CommandMeta> {
    REGISTRY.with(|r| r.borrow().entries.iter().map(|e| e.meta.clone()).collect())
}

/// Commands matching `query` (case-insensitive substring of title or category),
/// ranked: title prefix, then title substring, then category substring.
pub fn search(query: &str) -> Vec<CommandMeta> {
    let q = query.trim().to_lowercase();
    let mut scored: Vec<(i32, CommandMeta)> = REGISTRY.with(|r| {
        r.borrow()
            .entries
            .iter()
            .filter(|e| in_scope(&e.meta.context))
            .filter_map(|e| {
                if q.is_empty() {
                    return Some((0, e.meta.clone()));
                }
                let title = e.meta.title.to_lowercase();
                let cat = e.meta.category.to_lowercase();
                if title.starts_with(&q) {
                    Some((3, e.meta.clone()))
                } else if title.contains(&q) {
                    Some((2, e.meta.clone()))
                } else if cat.contains(&q) {
                    Some((1, e.meta.clone()))
                } else {
                    None
                }
            })
            .collect()
    });
    // Stable sort by descending score keeps registration order within a tier.
    scored.sort_by(|a, b| b.0.cmp(&a.0));
    scored.into_iter().map(|(_, m)| m).collect()
}

/// Bind a keyboard chord (e.g. `"Ctrl+S"`) to a command id.
pub fn bind_shortcut(chord: &str, id: &str) {
    REGISTRY.with(|r| {
        r.borrow_mut()
            .shortcuts
            .insert(normalize_chord(chord), id.to_string());
    });
}

/// The command id bound to `chord`, if any **and reachable in the active
/// context** — so a panel-scoped shortcut doesn't fire in the main window.
pub fn command_for_chord(chord: &str) -> Option<String> {
    REGISTRY.with(|r| {
        let r = r.borrow();
        let id = r.shortcuts.get(&normalize_chord(chord))?;
        let entry = r.entries.iter().find(|e| &e.meta.id == id)?;
        in_scope(&entry.meta.context).then(|| id.clone())
    })
}

/// Canonicalize a chord string for lookup: sorted modifier order, lower-cased.
fn normalize_chord(chord: &str) -> String {
    let mut ctrl = false;
    let mut shift = false;
    let mut alt = false;
    let mut meta = false;
    let mut key = String::new();
    for part in chord.split('+') {
        match part.trim().to_lowercase().as_str() {
            "ctrl" | "control" => ctrl = true,
            "shift" => shift = true,
            "alt" | "option" => alt = true,
            "meta" | "super" | "cmd" | "command" | "win" => meta = true,
            other => key = other.to_string(),
        }
    }
    let mut out = String::new();
    if ctrl {
        out.push_str("ctrl+");
    }
    if alt {
        out.push_str("alt+");
    }
    if shift {
        out.push_str("shift+");
    }
    if meta {
        out.push_str("meta+");
    }
    out.push_str(&key);
    out
}

/// Build the canonical chord string for a key + modifiers (matches
/// `normalize_chord`'s output).
pub fn chord_of(key: &Key, mods: Modifiers) -> String {
    let key_name = match key {
        Key::Function(n) => format!("f{n}"),
        Key::Character(c) => c.to_lowercase().to_string(),
        Key::Enter => "enter".into(),
        Key::Escape => "escape".into(),
        Key::Space => "space".into(),
        Key::Tab => "tab".into(),
        Key::Backspace => "backspace".into(),
        Key::Delete => "delete".into(),
        Key::Left => "left".into(),
        Key::Right => "right".into(),
        Key::Up => "up".into(),
        Key::Down => "down".into(),
        Key::Home => "home".into(),
        Key::End => "end".into(),
        Key::PageUp => "pageup".into(),
        Key::PageDown => "pagedown".into(),
        Key::Named(s) => s.to_lowercase(),
    };
    let mut out = String::new();
    if mods.ctrl {
        out.push_str("ctrl+");
    }
    if mods.alt {
        out.push_str("alt+");
    }
    if mods.shift {
        out.push_str("shift+");
    }
    if mods.meta {
        out.push_str("meta+");
    }
    out.push_str(&key_name);
    out
}

// ---------------------------------------------------------------------------
// Command Palette
// ---------------------------------------------------------------------------

const MAX_VISIBLE: usize = 10;

/// The built-in fuzzy command launcher (opened with `F1`). Owned and driven by
/// the [`App`](crate::App); applications only register commands.
pub struct CommandPalette {
    open: bool,
    query: String,
    selected: usize,
    results: Vec<CommandMeta>,
    font_size: f32,
    /// Geometry, recomputed each frame; shared by paint and hit-testing.
    panel: Rect,
    box_rect: Rect,
    row_rects: Vec<Rect>,
}

impl CommandPalette {
    /// A closed palette. [`App`](crate::App) already owns one — construct your
    /// own only if you drive the widget tree yourself.
    pub fn new() -> Self {
        CommandPalette {
            open: false,
            query: String::new(),
            selected: 0,
            results: Vec::new(),
            font_size: 15.0,
            panel: Rect::ZERO,
            box_rect: Rect::ZERO,
            row_rects: Vec::new(),
        }
    }

    /// Whether the palette is showing — and therefore swallowing input, since an
    /// open palette takes keys and clicks before the widget tree sees them.
    pub fn is_open(&self) -> bool {
        self.open
    }

    /// Open (refreshing the list) or close the palette.
    pub fn toggle(&mut self) {
        self.open = !self.open;
        if self.open {
            self.query.clear();
            self.selected = 0;
            self.refresh();
        }
    }

    fn close(&mut self) {
        self.open = false;
    }

    fn refresh(&mut self) {
        self.results = search(&self.query);
        if self.selected >= self.results.len() {
            self.selected = self.results.len().saturating_sub(1);
        }
    }

    /// Handle a key while open. Returns `true` if the palette consumed it.
    pub fn on_key(&mut self, key: &Key, _mods: Modifiers) -> bool {
        if !self.open {
            return false;
        }
        match key {
            Key::Escape => self.close(),
            Key::Enter => {
                if let Some(cmd) = self.results.get(self.selected) {
                    let id = cmd.id.clone();
                    self.close();
                    run(&id);
                } else {
                    self.close();
                }
            }
            Key::Down => {
                if !self.results.is_empty() {
                    self.selected = (self.selected + 1).min(self.results.len() - 1);
                }
            }
            Key::Up => {
                self.selected = self.selected.saturating_sub(1);
            }
            Key::Backspace => {
                self.query.pop();
                self.selected = 0;
                self.refresh();
            }
            _ => return false,
        }
        true
    }

    /// Handle committed text while open.
    pub fn on_text(&mut self, text: &str) {
        if !self.open {
            return;
        }
        // Ignore control characters (Enter/Escape arrive as keys).
        for ch in text.chars().filter(|c| !c.is_control()) {
            self.query.push(ch);
        }
        self.selected = 0;
        self.refresh();
    }

    /// Recompute the panel/search-box/row rects. Shared by `paint` and pointer
    /// hit-testing, so clicking a row always matches what is drawn.
    fn compute(&mut self, fonts: &Fonts, screen: Size) {
        let width = (screen.width * 0.6).clamp(360.0, 640.0);
        let x = (screen.width - width) * 0.5;
        let y = (screen.height * 0.14).max(24.0);
        let line_h = fonts.line_height(self.font_size, FontId::Ui);
        let box_h = line_h + 20.0;
        let row_h = line_h + 12.0;
        let visible = self.results.len().min(MAX_VISIBLE);
        // With no results we still draw a "No matching commands" line, so the
        // panel needs a row's worth of height for it — otherwise the message
        // falls outside the border.
        let list_h = if self.results.is_empty() {
            row_h
        } else {
            visible as f32 * row_h
        };
        let panel_h = box_h + list_h + 8.0;

        self.panel = Rect::from_xywh(x, y, width, panel_h);
        self.box_rect = Rect::from_xywh(x + 8.0, y + 8.0, width - 16.0, box_h - 8.0);
        self.row_rects = (0..visible)
            .map(|i| Rect::from_xywh(x + 6.0, y + box_h + i as f32 * row_h, width - 12.0, row_h))
            .collect();
    }

    /// Handle a pointer event while open: hover highlights a row, a click runs
    /// it, and a click outside dismisses. Returns whether the palette consumed it.
    pub fn on_pointer(
        &mut self,
        fonts: &Fonts,
        screen: Size,
        event: &crate::event::InputEvent,
    ) -> bool {
        use crate::event::{InputEvent, PointerButton};
        if !self.open {
            return false;
        }
        self.compute(fonts, screen);

        match event {
            InputEvent::PointerMoved { pos } => {
                if let Some(i) = self.row_rects.iter().position(|r| r.contains(*pos)) {
                    self.selected = i;
                }
                true
            }
            InputEvent::PointerPressed {
                pos,
                button: PointerButton::Primary,
            } => {
                if let Some(i) = self.row_rects.iter().position(|r| r.contains(*pos)) {
                    if let Some(cmd) = self.results.get(i) {
                        let id = cmd.id.clone();
                        self.close();
                        run(&id);
                    }
                } else if !self.panel.contains(*pos) {
                    self.close(); // click outside dismisses
                }
                true
            }
            InputEvent::PointerReleased { .. } | InputEvent::Scroll { .. } => true,
            _ => false,
        }
    }

    /// Paint the palette into the scene's overlay layer. `screen` is the logical
    /// window size.
    pub fn paint(&mut self, fonts: &Fonts, theme: &Theme, screen: Size, scene: &mut Scene) {
        if !self.open {
            return;
        }
        self.compute(fonts, screen);
        let p = &theme.palette;
        scene.begin_overlay();

        // Dim backdrop.
        scene.rect(
            Rect::from_xywh(0.0, 0.0, screen.width, screen.height),
            Color::rgba(0.0, 0.0, 0.0, 0.45),
        );

        let width = self.panel.width();
        let x = self.panel.left();
        let y = self.panel.top();
        let line_h = fonts.line_height(self.font_size, FontId::Ui);
        let box_h = line_h + 20.0;
        let row_h = line_h + 12.0;

        let panel = self.panel;
        scene.push_rect(
            RectShape::fill(panel, p.surface)
                .with_corner_radius(theme.radius.lg)
                .with_border(1.0, p.border),
        );

        // Search box.
        let box_rect = self.box_rect;
        scene.push_rect(
            RectShape::fill(box_rect, p.surface_variant)
                .with_corner_radius(theme.radius.md)
                .with_border(1.0, p.accent),
        );
        let text_y = box_rect.top()
            + (box_rect.height() - fonts.line_height(self.font_size, FontId::Ui)) * 0.5;
        if self.query.is_empty() {
            scene.text(
                Point::new(box_rect.left() + 10.0, text_y),
                "Type a command…",
                self.font_size,
                p.text_muted,
            );
        } else {
            let qw = fonts.measure(&self.query, self.font_size, FontId::Ui).width;
            scene.text(
                Point::new(box_rect.left() + 10.0, text_y),
                self.query.clone(),
                self.font_size,
                p.text,
            );
            // Caret.
            scene.rect(
                Rect::from_xywh(
                    box_rect.left() + 10.0 + qw + 1.0,
                    text_y,
                    1.5,
                    fonts.line_height(self.font_size, FontId::Ui),
                ),
                p.accent,
            );
        }

        // Results.
        let list_top = y + box_h;
        for (i, cmd) in self.results.iter().take(MAX_VISIBLE).enumerate() {
            let ry = list_top + i as f32 * row_h;
            let row = Rect::from_xywh(x + 6.0, ry, width - 12.0, row_h);
            if i == self.selected {
                scene.rounded_rect(row, p.selection, theme.radius.sm);
            }
            let ty = ry + (row_h - fonts.line_height(self.font_size, FontId::Ui)) * 0.5;
            let mut tx = row.left() + 10.0;
            if let Some(icon) = cmd.icon {
                let color = cmd.color.unwrap_or(p.text);
                scene.text_font(
                    Point::new(tx, ty),
                    icon.ch().to_string(),
                    self.font_size,
                    color,
                    icon.font_id(),
                );
                tx += fonts.char_advance(icon.ch(), self.font_size, icon.font_id()) + 8.0;
            }
            scene.text(
                Point::new(tx, ty),
                cmd.title.clone(),
                self.font_size,
                p.text,
            );

            // Right-aligned: shortcut, then category.
            let mut rx = row.right() - 12.0;
            if let Some(sc) = &cmd.shortcut {
                let w = fonts.measure(sc, self.font_size - 1.0, FontId::Ui).width;
                rx -= w;
                scene.text(
                    Point::new(rx, ty),
                    sc.clone(),
                    self.font_size - 1.0,
                    p.text_muted,
                );
                rx -= 14.0;
            }
            let cw = fonts
                .measure(&cmd.category, self.font_size - 1.0, FontId::Ui)
                .width;
            rx -= cw;
            scene.text(
                Point::new(rx, ty),
                cmd.category.clone(),
                self.font_size - 1.0,
                p.accent,
            );
        }

        if self.results.is_empty() {
            scene.text(
                Point::new(
                    x + 16.0,
                    list_top + (row_h - fonts.line_height(self.font_size, FontId::Ui)) * 0.5,
                ),
                "No matching commands",
                self.font_size,
                p.text_muted,
            );
        }

        scene.end_overlay();
    }
}

impl Default for CommandPalette {
    fn default() -> Self {
        CommandPalette::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;
    use std::rc::Rc;

    #[test]
    fn register_run_and_search() {
        // Use a unique id space to avoid cross-test interference.
        let ran = Rc::new(Cell::new(0));
        let r2 = ran.clone();
        register(
            CommandMeta::new("test.inc", "Increment Counter").category("Test"),
            move || r2.set(r2.get() + 1),
        );
        run("test.inc");
        assert_eq!(ran.get(), 1);

        let hits = search("increment");
        assert!(hits.iter().any(|m| m.id == "test.inc"));
        let none = search("zzz-nope-zzz");
        assert!(!none.iter().any(|m| m.id == "test.inc"));
    }

    /// A detached panel window shows its own commands; the main window does not.
    #[test]
    fn context_scopes_palette_visibility_and_shortcuts() {
        register(
            CommandMeta::new("ctx.global", "Global Thing").shortcut("Ctrl+F9"),
            || {},
        );
        register(
            CommandMeta::new("ctx.panel", "Panel Thing")
                .context("panel")
                .shortcut("Ctrl+F10"),
            || {},
        );

        // No context active (e.g. the main window): only global commands.
        set_active_context(None);
        let hits = search("Thing");
        assert!(hits.iter().any(|m| m.id == "ctx.global"));
        assert!(
            !hits.iter().any(|m| m.id == "ctx.panel"),
            "panel-scoped command must not leak into the global palette"
        );
        assert_eq!(
            command_for_chord("ctrl+f10"),
            None,
            "its shortcut must not fire either"
        );

        // Inside a panel window: global commands PLUS the panel-scoped ones.
        set_active_context(Some("panel".into()));
        let hits = search("Thing");
        assert!(hits.iter().any(|m| m.id == "ctx.panel"));
        assert!(hits.iter().any(|m| m.id == "ctx.global"));
        assert_eq!(command_for_chord("ctrl+f10").as_deref(), Some("ctx.panel"));

        set_active_context(None);
    }

    /// Regression: with no results the palette still draws a "No matching
    /// commands" line, so its panel must be tall enough to enclose it.
    #[test]
    fn empty_palette_panel_encloses_the_no_matches_line() {
        let Some(fonts) = crate::text::Fonts::load() else {
            return;
        };
        let mut palette = CommandPalette::new();
        palette.toggle(); // open
        palette.query = "zzz-definitely-no-such-command-zzz".into();
        palette.refresh();
        assert!(palette.results.is_empty());

        palette.compute(&fonts, Size::new(1000.0, 700.0));

        let line_h = fonts.line_height(palette.font_size, FontId::Ui);
        let search_box_h = line_h + 20.0;
        let row_h = line_h + 12.0;
        assert!(
            palette.panel.height() >= search_box_h + row_h,
            "panel ({}) must leave a row for the empty-state message",
            palette.panel.height()
        );
        // ...and the search box must sit inside it.
        assert!(palette.panel.bottom() > palette.box_rect.bottom());
    }

    #[test]
    fn shortcut_normalization_and_lookup() {
        register(
            CommandMeta::new("test.save", "Save").shortcut("Ctrl+S"),
            || {},
        );
        assert_eq!(command_for_chord("ctrl+s").as_deref(), Some("test.save"));
        // Modifier order / case doesn't matter.
        assert_eq!(
            chord_of(
                &Key::Character('S'),
                Modifiers {
                    ctrl: true,
                    ..Default::default()
                }
            ),
            "ctrl+s"
        );
    }
}
