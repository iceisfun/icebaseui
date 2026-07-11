//! The application shell: window creation and the winit event loop.
//!
//! [`App`] is the entry point an application uses to get a themed window on
//! screen. Each frame it clears to the theme background, invokes the
//! application's UI callback to populate a [`Scene`], and renders that scene.
//!
//! The UI callback is the temporary seam for this milestone: it hands the app a
//! raw [`Scene`] plus a [`Frame`] context (logical size + theme). Later
//! milestones replace it with the retained widget tree, but the window/event/
//! render plumbing established here stays the same.

use std::sync::Arc;

use baseui_core::Size;
use baseui_core::paint::Scene;
use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::window::{Window, WindowId};

use crate::render::Renderer;
use crate::theme::Theme;

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

/// Per-frame context passed to the application's UI callback.
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

/// The BaseUI application. Build one with [`App::new`], configure it with the
/// builder methods, set a UI callback with [`App::on_frame`], then [`App::run`].
pub struct App {
    config: WindowConfig,
    theme: Theme,
    ui: Option<UiFn>,
    scene: Scene,
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
            ui: None,
            scene: Scene::new(),
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

    /// Set the per-frame UI callback. It receives a fresh (cleared) [`Scene`] to
    /// populate and a [`Frame`] describing the surface.
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

    /// Rebuild the scene from the UI callback and draw a frame.
    fn redraw(&mut self, event_loop: &ActiveEventLoop) {
        let Some(state) = self.state.as_mut() else {
            return;
        };

        self.scene.clear();
        if let Some(ui) = self.ui.as_mut() {
            let frame = Frame {
                size: state.renderer.logical_size(),
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

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.state.is_some() {
            return;
        }

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

        match Renderer::new(window.clone()) {
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
            WindowEvent::RedrawRequested => {
                self.redraw(event_loop);
            }
            _ => {}
        }
    }
}
