//! The application shell: window creation, the winit event loop, and driving
//! the retained widget tree.
//!
//! [`App`] gets a themed window on screen and, each frame, runs the widget
//! tree's layout → paint passes into a [`Scene`], then renders it. Raw winit
//! input is normalized into [`InputEvent`]s (logical coordinates) and routed to
//! the tree. Signal writes from event handlers schedule repaints through the
//! reactive change hook registered here.
//!
//! Two ways to describe UI are supported:
//! - [`App::with_root`] — a retained [`Widget`] tree (the normal path).
//! - [`App::on_frame`] — a raw per-frame [`Scene`] callback (handy for custom
//!   painting / demos before a widget exists for something).

use std::rc::Rc;
use std::sync::Arc;

use baseui_core::paint::Scene;
use baseui_core::reactive;
use baseui_core::{Point, Rect, Size, Vec2};
use winit::application::ApplicationHandler;
use winit::event::{ElementState, MouseButton, MouseScrollDelta, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::window::{Window, WindowId};

use crate::event::{InputEvent, PointerButton};
use crate::layout::Constraints;
use crate::render::Renderer;
use crate::text::Fonts;
use crate::theme::Theme;
use crate::widget::{EventCx, LayoutCx, PaintCx, Widget};

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

/// Per-frame context passed to a raw [`App::on_frame`] callback.
pub struct Frame<'a> {
    /// The drawable surface size in logical pixels.
    pub size: Size,
    /// The active theme.
    pub theme: &'a Theme,
}

type UiFn = Box<dyn FnMut(&mut Scene, &Frame<'_>)>;

/// Live per-window state: window handle and its renderer.
struct WindowState {
    window: Arc<Window>,
    renderer: Renderer,
}

/// The BaseUI application. Build with [`App::new`], configure with the builders,
/// attach UI via [`App::with_root`] or [`App::on_frame`], then [`App::run`].
pub struct App {
    config: WindowConfig,
    theme: Theme,
    root: Option<Box<dyn Widget>>,
    ui: Option<UiFn>,
    scene: Scene,
    fonts: Option<Rc<Fonts>>,
    pointer: Point,
    state: Option<WindowState>,
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
            theme: Theme::default(),
            root: None,
            ui: None,
            scene: Scene::new(),
            fonts: None,
            pointer: Point::ZERO,
            state: None,
        }
    }

    /// Set the window title.
    pub fn with_title(mut self, title: impl Into<String>) -> Self {
        self.config.title = title.into();
        self
    }

    /// Set the initial window size in logical pixels.
    pub fn with_size(mut self, width: u32, height: u32) -> Self {
        self.config.width = width;
        self.config.height = height;
        self
    }

    /// Set the active theme.
    pub fn with_theme(mut self, theme: Theme) -> Self {
        self.theme = theme;
        self
    }

    /// Attach a retained widget tree as the application root.
    pub fn with_root(mut self, root: impl Widget + 'static) -> Self {
        self.root = Some(Box::new(root));
        self
    }

    /// Set a raw per-frame scene callback (used when there is no widget root).
    pub fn on_frame(mut self, ui: impl FnMut(&mut Scene, &Frame<'_>) + 'static) -> Self {
        self.ui = Some(Box::new(ui));
        self
    }

    /// Run the application. Blocks until the window is closed.
    pub fn run(mut self) -> Result<(), Box<dyn std::error::Error>> {
        let event_loop = EventLoop::new()?;
        event_loop.set_control_flow(ControlFlow::Wait);
        event_loop.run_app(&mut self)?;
        Ok(())
    }

    /// The whole-window bounds in logical pixels.
    fn root_bounds(&self) -> Rect {
        match self.state.as_ref() {
            Some(state) => {
                let s = state.renderer.logical_size();
                Rect::from_xywh(0.0, 0.0, s.width, s.height)
            }
            None => Rect::ZERO,
        }
    }

    /// Route a normalized input event to the widget tree, then request a redraw
    /// (interaction may have changed visual state or signal-backed state).
    fn dispatch(&mut self, event: InputEvent) {
        let bounds = self.root_bounds();
        if let (Some(root), Some(fonts)) = (self.root.as_mut(), self.fonts.as_ref()) {
            let mut cx = EventCx {
                fonts,
                theme: &self.theme,
            };
            root.event(&mut cx, bounds, &event);
        }
        if let Some(state) = self.state.as_ref() {
            state.window.request_redraw();
        }
    }

    /// Rebuild the scene (widget tree or raw callback) and draw a frame.
    fn redraw(&mut self, event_loop: &ActiveEventLoop) {
        let Some(state) = self.state.as_mut() else {
            return;
        };
        let logical = state.renderer.logical_size();
        self.scene.clear();

        if let (Some(root), Some(fonts)) = (self.root.as_mut(), self.fonts.as_ref()) {
            let mut lcx = LayoutCx {
                fonts,
                theme: &self.theme,
            };
            let size = root.layout(&mut lcx, Constraints::loose(logical));
            let bounds = Rect::new(Point::ZERO, size);
            let mut pcx = PaintCx {
                fonts,
                theme: &self.theme,
            };
            root.paint(&mut pcx, bounds, &mut self.scene);
        } else if let Some(ui) = self.ui.as_mut() {
            let frame = Frame {
                size: logical,
                theme: &self.theme,
            };
            ui(&mut self.scene, &frame);
        }

        if let Err(e) = state
            .renderer
            .render(&self.scene, self.theme.palette.background)
        {
            log::error!("render error: {e}");
            event_loop.exit();
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

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.state.is_some() {
            return;
        }

        // Load fonts once; shared between layout (measurement) and the renderer
        // (rasterization).
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
        let fonts = self.fonts.clone().unwrap();

        let attributes = Window::default_attributes()
            .with_title(self.config.title.clone())
            .with_inner_size(winit::dpi::LogicalSize::new(
                self.config.width as f64,
                self.config.height as f64,
            ));

        let window = match event_loop.create_window(attributes) {
            Ok(window) => Arc::new(window),
            Err(e) => {
                log::error!("failed to create window: {e}");
                event_loop.exit();
                return;
            }
        };

        // Reactive → repaint bridge: any signal write requests a redraw.
        let redraw_target = window.clone();
        reactive::set_on_change(move || redraw_target.request_redraw());

        match Renderer::new(window.clone(), fonts) {
            Ok(renderer) => {
                window.request_redraw();
                self.state = Some(WindowState { window, renderer });
            }
            Err(e) => {
                log::error!("failed to initialize renderer: {e}");
                event_loop.exit();
            }
        }
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        window_id: WindowId,
        event: WindowEvent,
    ) {
        {
            let Some(state) = self.state.as_ref() else {
                return;
            };
            if state.window.id() != window_id {
                return;
            }
        }

        match event {
            WindowEvent::CloseRequested => {
                event_loop.exit();
            }
            WindowEvent::Resized(new_size) => {
                if let Some(state) = self.state.as_mut() {
                    state.renderer.resize(new_size);
                    state.window.request_redraw();
                }
            }
            WindowEvent::ScaleFactorChanged { scale_factor, .. } => {
                if let Some(state) = self.state.as_mut() {
                    state.renderer.set_scale_factor(scale_factor);
                    state.window.request_redraw();
                }
            }
            WindowEvent::CursorMoved { position, .. } => {
                let scale = self
                    .state
                    .as_ref()
                    .map(|s| s.renderer.scale_factor())
                    .unwrap_or(1.0) as f64;
                self.pointer = Point::new(
                    (position.x / scale) as f32,
                    (position.y / scale) as f32,
                );
                let pos = self.pointer;
                self.dispatch(InputEvent::PointerMoved { pos });
            }
            WindowEvent::CursorLeft { .. } => {
                self.dispatch(InputEvent::PointerLeft);
            }
            WindowEvent::MouseInput { state, button, .. } => {
                if let Some(button) = map_button(button) {
                    let pos = self.pointer;
                    let event = match state {
                        ElementState::Pressed => InputEvent::PointerPressed { pos, button },
                        ElementState::Released => InputEvent::PointerReleased { pos, button },
                    };
                    self.dispatch(event);
                }
            }
            WindowEvent::MouseWheel { delta, .. } => {
                // Normalize both delta kinds to approximate "lines".
                let delta = match delta {
                    MouseScrollDelta::LineDelta(x, y) => Vec2::new(x, y),
                    MouseScrollDelta::PixelDelta(p) => {
                        Vec2::new(p.x as f32 / 16.0, p.y as f32 / 16.0)
                    }
                };
                let pos = self.pointer;
                self.dispatch(InputEvent::Scroll { pos, delta });
            }
            WindowEvent::RedrawRequested => {
                self.redraw(event_loop);
            }
            _ => {}
        }
    }
}
