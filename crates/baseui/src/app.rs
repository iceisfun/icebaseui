//! The application shell: window creation and the winit event loop.
//!
//! [`App`] is the entry point an application uses to get a themed window on
//! screen. For this foundation milestone it opens a single window and clears it
//! to the active theme's background color. The event loop, window lifecycle, and
//! renderer ownership established here are what later milestones (layout, widget
//! tree, input routing) will hook into.

use std::sync::Arc;

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

/// Live per-window state: the window handle and its renderer. Created lazily on
/// the first `resumed` event, per winit 0.30's application model.
struct WindowState {
    window: Arc<Window>,
    renderer: Renderer,
}

/// The BaseUI application. Build one with [`App::new`], configure it with the
/// builder methods, then call [`App::run`].
pub struct App {
    config: WindowConfig,
    theme: Theme,
    state: Option<WindowState>,
}

impl Default for App {
    fn default() -> Self {
        App::new()
    }
}

impl App {
    /// Create an application with default configuration and the default (dark)
    /// theme.
    pub fn new() -> Self {
        App {
            config: WindowConfig::default(),
            theme: Theme::default(),
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

    /// Run the application. Blocks until the window is closed.
    pub fn run(mut self) -> Result<(), Box<dyn std::error::Error>> {
        let event_loop = EventLoop::new()?;
        // Wait for events rather than spinning; this is a conventional desktop
        // app, not a game loop. Redraws are requested explicitly when needed.
        event_loop.set_control_flow(ControlFlow::Wait);
        event_loop.run_app(&mut self)?;
        Ok(())
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
        let Some(state) = self.state.as_mut() else {
            return;
        };
        if state.window.id() != window_id {
            return;
        }

        match event {
            WindowEvent::CloseRequested => {
                event_loop.exit();
            }
            WindowEvent::Resized(new_size) => {
                state.renderer.resize(new_size);
                state.window.request_redraw();
            }
            WindowEvent::RedrawRequested => {
                if let Err(e) = state.renderer.render(self.theme.palette.background) {
                    log::error!("render error: {e}");
                    event_loop.exit();
                }
            }
            _ => {}
        }
    }
}
