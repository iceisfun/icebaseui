//! The application shell: window management, the winit event loop, and driving
//! the retained widget tree.
//!
//! [`App`] drives **any number of windows** from one GPU device and one event
//! loop. Each window owns its own root widget, [`Scene`], and pointer state;
//! they share the [`GpuContext`], the glyph atlas, the theme, the command
//! registry, and the reactive runtime. A signal write repaints every window.
//!
//! Secondary windows are opened by queueing a [`WindowSpec`](crate::window::WindowSpec)
//! with [`window::open`](crate::window::open) — from a command handler, a button,
//! or (later) a dock tab tear-off. The queue is drained when the event loop goes
//! idle, which is the only place the `ActiveEventLoop` is available.

use std::rc::Rc;
use std::sync::Arc;

use baseui_core::paint::Scene;
use baseui_core::reactive;
use baseui_core::{Point, Rect, Size, Vec2};
use winit::application::ApplicationHandler;
use winit::event::{ElementState, MouseButton, MouseScrollDelta, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::keyboard::{Key as WKey, NamedKey};
use winit::window::{Window, WindowId};

use crate::event::{InputEvent, Key, Modifiers, PointerButton};
use crate::layout::Constraints;
use crate::render::{GpuContext, WindowRenderer};
use crate::text::Fonts;
use crate::theme::Theme;
use crate::widget::{EventCx, LayoutCx, PaintCx, Widget};
use crate::window;

/// Configuration for the main application window.
#[derive(Clone, Debug)]
pub struct WindowConfig {
    pub title: String,
    pub width: u32,
    pub height: u32,
}

impl Default for WindowConfig {
    fn default() -> Self {
        WindowConfig {
            title: "BaseUI".to_string(),
            width: 1280,
            height: 800,
        }
    }
}

/// Per-frame context passed to a raw [`App::on_frame`] callback (main window).
pub struct Frame<'a> {
    /// The drawable surface size in logical pixels.
    pub size: Size,
    /// The active theme.
    pub theme: &'a Theme,
}

type UiFn = Box<dyn FnMut(&mut Scene, &Frame<'_>)>;

/// One live window: its handle, renderer, root widget, scene, and pointer.
struct WindowState {
    window: Arc<Window>,
    renderer: WindowRenderer,
    root: Option<Box<dyn Widget>>,
    scene: Scene,
    pointer: Point,
    /// Command context this window activates while focused (scopes the palette
    /// and shortcuts). `None` = the global set only.
    context: Option<String>,
    /// The main window; closing it exits the app.
    is_main: bool,
}

/// The BaseUI application. Build with [`App::new`], configure with the builders,
/// attach UI via [`App::with_root`] or [`App::on_frame`], then [`App::run`].
pub struct App {
    config: WindowConfig,
    /// The theme as configured; `theme` is this scaled by the global text scale.
    base_theme: Theme,
    theme: Theme,
    /// Text scale the active `theme` was derived at.
    applied_scale: f32,

    /// Root for the main window, held until it is created.
    pending_root: Option<Box<dyn Widget>>,
    ui: Option<UiFn>,

    fonts: Option<Rc<Fonts>>,
    gpu: Option<GpuContext>,
    windows: Vec<WindowState>,
    /// The focused window — where the command palette is drawn.
    active: Option<WindowId>,

    modifiers: Modifiers,
    palette: crate::command::CommandPalette,
    persist_path: Option<std::path::PathBuf>,
    store: crate::persist::Store,
}

impl Default for App {
    fn default() -> Self {
        App::new()
    }
}

impl App {
    /// Create an application with default configuration and the dark theme.
    pub fn new() -> Self {
        App {
            config: WindowConfig::default(),
            base_theme: Theme::default(),
            theme: Theme::default(),
            applied_scale: 1.0,
            pending_root: None,
            ui: None,
            fonts: None,
            gpu: None,
            windows: Vec::new(),
            active: None,
            modifiers: Modifiers::default(),
            palette: crate::command::CommandPalette::new(),
            persist_path: None,
            store: crate::persist::Store::new(),
        }
    }

    /// Persist and restore UI state (split sizes, active tabs, group collapse,
    /// tree expansion, scroll offsets, text scale) and main-window geometry.
    pub fn with_persistence(mut self, path: impl Into<std::path::PathBuf>) -> Self {
        self.persist_path = Some(path.into());
        self
    }

    /// Set the main window title.
    pub fn with_title(mut self, title: impl Into<String>) -> Self {
        self.config.title = title.into();
        self
    }

    /// Set the initial main-window size in logical pixels.
    pub fn with_size(mut self, width: u32, height: u32) -> Self {
        self.config.width = width;
        self.config.height = height;
        self
    }

    /// Set the active theme.
    pub fn with_theme(mut self, theme: Theme) -> Self {
        self.base_theme = theme.clone();
        self.theme = theme;
        self
    }

    /// Set the initial global text scale (`1.0` = 100%).
    pub fn with_text_scale(self, scale: f32) -> Self {
        crate::text::set_scale(scale);
        self
    }

    /// Attach a retained widget tree as the main window's root.
    pub fn with_root(mut self, root: impl Widget + 'static) -> Self {
        self.pending_root = Some(Box::new(root));
        self
    }

    /// Set a raw per-frame scene callback for the main window (used when there
    /// is no widget root).
    pub fn on_frame(mut self, ui: impl FnMut(&mut Scene, &Frame<'_>) + 'static) -> Self {
        self.ui = Some(Box::new(ui));
        self
    }

    /// Run the application. Blocks until the main window is closed.
    pub fn run(mut self) -> Result<(), Box<dyn std::error::Error>> {
        let event_loop = EventLoop::new()?;
        event_loop.set_control_flow(ControlFlow::Wait);
        event_loop.run_app(&mut self)?;
        Ok(())
    }

    /// Re-derive the active theme when the global text scale changed.
    fn refresh_theme(&mut self) {
        let scale = crate::text::scale();
        if (scale - self.applied_scale).abs() > f32::EPSILON {
            self.theme = self.base_theme.scaled(scale);
            self.applied_scale = scale;
        }
    }

    fn index_of(&self, id: WindowId) -> Option<usize> {
        self.windows.iter().position(|w| w.window.id() == id)
    }

    /// Focus moved: publish the focused window and activate its command context,
    /// so the palette and shortcuts scope to it.
    fn set_active(&mut self, id: Option<WindowId>) {
        self.active = id;
        window::set_focused(id);
        let context = id
            .and_then(|id| self.index_of(id))
            .and_then(|i| self.windows[i].context.clone());
        crate::command::set_active_context(context);
    }

    fn request_redraw_all(&self) {
        for state in &self.windows {
            state.window.request_redraw();
        }
    }

    /// Route a normalized input event to one window's widget tree.
    fn dispatch(&mut self, index: usize, event: InputEvent) {
        self.refresh_theme();
        let logical = self.windows[index].renderer.logical_size();
        let bounds = Rect::from_xywh(0.0, 0.0, logical.width, logical.height);
        let window_id = self.windows[index].window.id();

        if let (Some(root), Some(fonts)) = (
            self.windows[index].root.as_mut(),
            self.fonts.as_ref(),
        ) {
            let mut cx = EventCx::new(fonts, &self.theme, logical).with_window(window_id);
            root.event(&mut cx, bounds, &event);
        }
        self.windows[index].window.request_redraw();
    }

    /// Route a keyboard event: the open command palette first, then global
    /// shortcuts, then the focused widget in the window that received it.
    fn handle_keyboard(
        &mut self,
        index: usize,
        key: Option<Key>,
        pressed: bool,
        text: Option<String>,
    ) {
        if self.palette.is_open() {
            if pressed {
                if let Some(k) = &key {
                    self.palette.on_key(k, self.modifiers);
                }
            }
            if let Some(t) = &text {
                self.palette.on_text(t);
            }
            self.request_redraw_all();
            return;
        }

        if pressed {
            if let Some(k) = &key {
                let chord = crate::command::chord_of(k, self.modifiers);
                if chord == "f1" || chord == "ctrl+shift+p" {
                    self.palette.toggle();
                    self.request_redraw_all();
                    return;
                }
                // A focused text field takes plain keys; modified chords still
                // fire shortcuts. An open popup is modal and suppresses both.
                let focused = crate::focus::current().is_some();
                let modified = self.modifiers.ctrl || self.modifiers.alt || self.modifiers.meta;
                let popup = crate::popup::is_open();
                if !popup && (!focused || modified) {
                    if let Some(id) = crate::command::command_for_chord(&chord) {
                        crate::command::run(&id);
                        self.request_redraw_all();
                        return;
                    }
                }
            }
        }

        if let Some(k) = key {
            self.dispatch(
                index,
                InputEvent::Key {
                    key: k,
                    pressed,
                    mods: self.modifiers,
                },
            );
        }
        if pressed {
            if let Some(t) = text {
                self.dispatch(index, InputEvent::Text { text: t });
            }
        }
    }

    /// Save main-window state and geometry to the persistence file, if enabled.
    fn save_state(&mut self) {
        if self.persist_path.is_none() {
            return;
        }
        if let Some(main) = self.windows.iter().find(|w| w.is_main) {
            if let Some(root) = main.root.as_ref() {
                root.persist_save(&mut self.store);
            }
            let size = main.renderer.logical_size();
            self.store.set("window.width", &(size.width as f64));
            self.store.set("window.height", &(size.height as f64));
        }
        self.store.set("ui.text_scale", &crate::text::scale());
        self.store.save();
    }

    /// Build and draw one window's frame.
    fn redraw(&mut self, index: usize, event_loop: &ActiveEventLoop) {
        self.refresh_theme();
        let Some(gpu) = self.gpu.as_mut() else {
            return;
        };
        let Some(fonts) = self.fonts.as_ref() else {
            return;
        };
        let state = &mut self.windows[index];
        let logical = state.renderer.logical_size();
        state.scene.clear();

        if let Some(root) = state.root.as_mut() {
            let mut lcx = LayoutCx {
                fonts,
                theme: &self.theme,
                window: Some(state.window.id()),
            };
            let size = root.layout(&mut lcx, Constraints::loose(logical));
            let bounds = Rect::new(Point::ZERO, size);
            let mut pcx = PaintCx {
                fonts,
                theme: &self.theme,
                screen: logical,
            };
            root.paint(&mut pcx, bounds, &mut state.scene);
        } else if state.is_main {
            if let Some(ui) = self.ui.as_mut() {
                let frame = Frame {
                    size: logical,
                    theme: &self.theme,
                };
                ui(&mut state.scene, &frame);
            }
        }

        // The command palette floats above the *focused* window.
        let is_active = self.active == Some(state.window.id())
            || (self.active.is_none() && state.is_main);
        if is_active {
            self.palette
                .paint(fonts, &self.theme, logical, &mut state.scene);
        }

        if let Err(e) = gpu.render(
            &mut state.renderer,
            &state.scene,
            self.theme.palette.background,
        ) {
            log::error!("render error: {e}");
            event_loop.exit();
        }
    }

    /// Create a window and attach it to the app. `root` is `None` for the
    /// `on_frame` (raw scene) path.
    #[allow(clippy::too_many_arguments)]
    fn create_window(
        &mut self,
        event_loop: &ActiveEventLoop,
        title: &str,
        width: u32,
        height: u32,
        position: Option<(i32, i32)>,
        context: Option<String>,
        root: Option<Box<dyn Widget>>,
        is_main: bool,
    ) -> Option<WindowId> {
        let mut attributes = Window::default_attributes()
            .with_title(title.to_string())
            .with_inner_size(winit::dpi::LogicalSize::new(width as f64, height as f64));
        if let Some((x, y)) = position {
            attributes = attributes.with_position(winit::dpi::PhysicalPosition::new(x, y));
        }

        let window = match event_loop.create_window(attributes) {
            Ok(window) => Arc::new(window),
            Err(e) => {
                log::error!("failed to create window: {e}");
                return None;
            }
        };

        // The first window bootstraps the shared GPU context.
        let renderer = match self.gpu.as_ref() {
            Some(gpu) => match gpu.add_window(window.clone()) {
                Ok(renderer) => renderer,
                Err(e) => {
                    log::error!("failed to create surface for new window: {e}");
                    return None;
                }
            },
            None => {
                let fonts = self.fonts.clone()?;
                match GpuContext::new(window.clone(), fonts) {
                    Ok((gpu, renderer)) => {
                        self.gpu = Some(gpu);
                        renderer
                    }
                    Err(e) => {
                        log::error!("failed to initialize GPU: {e}");
                        event_loop.exit();
                        return None;
                    }
                }
            }
        };

        let id = window.id();
        window.request_redraw();
        self.windows.push(WindowState {
            window,
            renderer,
            root,
            scene: Scene::new(),
            pointer: Point::ZERO,
            context,
            is_main,
        });
        Some(id)
    }

    /// Drain queued open/close window requests (event loop idle).
    fn process_window_requests(&mut self, event_loop: &ActiveEventLoop) {
        for request in window::take_requests() {
            match request {
                window::Request::Open(spec) => {
                    self.create_window(
                        event_loop,
                        &spec.title,
                        spec.width,
                        spec.height,
                        spec.position,
                        spec.context,
                        Some(spec.root),
                        false,
                    );
                }
                window::Request::Close(id) => {
                    if let Some(i) = self.index_of(id) {
                        if self.windows[i].is_main {
                            self.save_state();
                            event_loop.exit();
                        } else {
                            self.windows.remove(i);
                        }
                    }
                }
            }
        }
    }
}

/// Map a winit mouse button to our [`PointerButton`], if we track it.
fn map_button(button: MouseButton) -> Option<PointerButton> {
    match button {
        MouseButton::Left => Some(PointerButton::Primary),
        MouseButton::Right => Some(PointerButton::Secondary),
        MouseButton::Middle => Some(PointerButton::Middle),
        _ => None,
    }
}

/// Map a winit logical key to our normalized [`Key`].
fn map_key(key: &WKey) -> Option<Key> {
    match key {
        WKey::Named(named) => Some(match named {
            NamedKey::Escape => Key::Escape,
            NamedKey::Enter => Key::Enter,
            NamedKey::Tab => Key::Tab,
            NamedKey::Backspace => Key::Backspace,
            NamedKey::Delete => Key::Delete,
            NamedKey::ArrowLeft => Key::Left,
            NamedKey::ArrowRight => Key::Right,
            NamedKey::ArrowUp => Key::Up,
            NamedKey::ArrowDown => Key::Down,
            NamedKey::Home => Key::Home,
            NamedKey::End => Key::End,
            NamedKey::PageUp => Key::PageUp,
            NamedKey::PageDown => Key::PageDown,
            NamedKey::Space => Key::Space,
            NamedKey::F1 => Key::Function(1),
            NamedKey::F2 => Key::Function(2),
            NamedKey::F3 => Key::Function(3),
            NamedKey::F4 => Key::Function(4),
            NamedKey::F5 => Key::Function(5),
            NamedKey::F6 => Key::Function(6),
            NamedKey::F7 => Key::Function(7),
            NamedKey::F8 => Key::Function(8),
            NamedKey::F9 => Key::Function(9),
            NamedKey::F10 => Key::Function(10),
            NamedKey::F11 => Key::Function(11),
            NamedKey::F12 => Key::Function(12),
            other => Key::Named(format!("{other:?}")),
        }),
        WKey::Character(s) => s.chars().next().map(Key::Character),
        _ => None,
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if !self.windows.is_empty() {
            return;
        }

        // Fonts, shared between layout (measurement) and the GPU (rasterization).
        if self.fonts.is_none() {
            match Fonts::load() {
                Some(fonts) => self.fonts = Some(fonts),
                None => {
                    log::error!("no usable system fonts found; cannot start");
                    event_loop.exit();
                    return;
                }
            }
        }

        // Persisted state (and window geometry / text scale) before creating it.
        if let Some(path) = &self.persist_path {
            self.store = crate::persist::Store::load(path);
            if let Some(scale) = self.store.get::<f32>("ui.text_scale") {
                crate::text::set_scale(scale);
            }
        }
        let mut width = self.config.width;
        let mut height = self.config.height;
        if let (Some(w), Some(h)) = (
            self.store.get::<f64>("window.width"),
            self.store.get::<f64>("window.height"),
        ) {
            if w >= 200.0 && h >= 150.0 {
                width = w as u32;
                height = h as u32;
            }
        }

        // Any signal write repaints *every* window.
        reactive::set_on_change(window::mark_dirty);

        let root = self.pending_root.take();
        let title = self.config.title.clone();
        let Some(id) =
            self.create_window(event_loop, &title, width, height, None, None, root, true)
        else {
            return;
        };
        self.set_active(Some(id));

        // Restore persisted widget state before the first layout.
        if self.persist_path.is_some() {
            if let Some(main) = self.windows.iter_mut().find(|w| w.is_main) {
                if let Some(root) = main.root.as_mut() {
                    root.persist_restore(&self.store);
                }
            }
        }

        if std::env::var_os("BASEUI_OPEN_PALETTE").is_some() {
            self.palette.toggle();
        }
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        window_id: WindowId,
        event: WindowEvent,
    ) {
        let Some(index) = self.index_of(window_id) else {
            return;
        };

        match event {
            WindowEvent::CloseRequested => {
                if self.windows[index].is_main {
                    self.save_state();
                    event_loop.exit();
                } else {
                    self.windows.remove(index);
                }
            }
            WindowEvent::Focused(true) => {
                self.set_active(Some(window_id));
            }
            WindowEvent::Resized(new_size) => {
                if let Some(gpu) = self.gpu.as_ref() {
                    self.windows[index].renderer.resize(gpu, new_size);
                }
                self.windows[index].window.request_redraw();
            }
            WindowEvent::ScaleFactorChanged { scale_factor, .. } => {
                self.windows[index]
                    .renderer
                    .set_scale_factor(scale_factor);
                self.windows[index].window.request_redraw();
            }
            WindowEvent::CursorMoved { position, .. } => {
                let scale = self.windows[index].renderer.scale_factor() as f64;
                let pos = Point::new((position.x / scale) as f32, (position.y / scale) as f32);
                self.windows[index].pointer = pos;
                self.dispatch(index, InputEvent::PointerMoved { pos });
            }
            WindowEvent::CursorLeft { .. } => {
                self.dispatch(index, InputEvent::PointerLeft);
            }
            WindowEvent::MouseInput { state, button, .. } => {
                if let Some(button) = map_button(button) {
                    let pos = self.windows[index].pointer;
                    let event = match state {
                        ElementState::Pressed => InputEvent::PointerPressed { pos, button },
                        ElementState::Released => InputEvent::PointerReleased { pos, button },
                    };
                    self.dispatch(index, event);
                }
            }
            WindowEvent::MouseWheel { delta, .. } => {
                let delta = match delta {
                    MouseScrollDelta::LineDelta(x, y) => Vec2::new(x, y),
                    MouseScrollDelta::PixelDelta(p) => {
                        Vec2::new(p.x as f32 / 16.0, p.y as f32 / 16.0)
                    }
                };
                let pos = self.windows[index].pointer;
                self.dispatch(index, InputEvent::Scroll { pos, delta });
            }
            WindowEvent::ModifiersChanged(mods) => {
                let s = mods.state();
                self.modifiers = Modifiers {
                    ctrl: s.control_key(),
                    shift: s.shift_key(),
                    alt: s.alt_key(),
                    meta: s.super_key(),
                };
            }
            WindowEvent::KeyboardInput { event, .. } => {
                let pressed = event.state == ElementState::Pressed;
                let key = map_key(&event.logical_key);
                let text = if pressed {
                    event.text.as_ref().map(|s| s.to_string())
                } else {
                    None
                };
                self.handle_keyboard(index, key, pressed, text);
            }
            WindowEvent::RedrawRequested => {
                self.redraw(index, event_loop);
            }
            _ => {}
        }
    }

    /// The event loop is going idle: this is the only place we hold an
    /// `ActiveEventLoop`, so queued windows are created here, and a dirty flag
    /// from a signal write repaints every window.
    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        self.process_window_requests(event_loop);
        if window::take_dirty() {
            self.request_redraw_all();
        }
        if self.windows.is_empty() {
            event_loop.exit();
        }
    }
}
