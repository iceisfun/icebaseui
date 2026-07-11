//! The wgpu rendering backend.
//!
//! For this foundation milestone the renderer does exactly one thing: acquire a
//! surface tied to a winit window and clear it to a color each frame. It owns
//! all long-lived GPU objects (instance, adapter, device, queue, surface) so
//! the rest of the framework can stay ignorant of wgpu.
//!
//! Later milestones will grow this into a batched 2D painter (rounded rects,
//! borders, clipping, glyph atlas) that consumes a display list produced by the
//! widget tree. The public surface here is deliberately small to keep that
//! evolution non-breaking.

use std::sync::Arc;

use baseui_core::Color;
use winit::window::Window;

/// Owns the GPU device and the window surface, and clears the surface to a
/// requested color each frame.
pub struct Renderer {
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    /// Current surface size in physical pixels.
    size: winit::dpi::PhysicalSize<u32>,
}

impl Renderer {
    /// Create a renderer for `window`. Blocks on GPU adapter/device acquisition;
    /// intended to be called once when the window is first created.
    pub fn new(window: Arc<Window>) -> Result<Self, RendererError> {
        pollster::block_on(Self::new_async(window))
    }

    async fn new_async(window: Arc<Window>) -> Result<Self, RendererError> {
        let mut size = window.inner_size();
        // A zero-sized surface is invalid; clamp to at least 1x1 until the first
        // real resize arrives.
        size.width = size.width.max(1);
        size.height = size.height.max(1);

        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
            backends: wgpu::Backends::PRIMARY,
            ..Default::default()
        });

        let surface = instance
            .create_surface(window.clone())
            .map_err(|e| RendererError::Surface(e.to_string()))?;

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::default(),
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .map_err(|e| RendererError::NoAdapter(e.to_string()))?;

        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: Some("baseui-device"),
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::default(),
                memory_hints: wgpu::MemoryHints::default(),
                trace: wgpu::Trace::Off,
            })
            .await
            .map_err(|e| RendererError::NoDevice(e.to_string()))?;

        let caps = surface.get_capabilities(&adapter);
        // Prefer an sRGB surface format so our sRGB colors display correctly.
        let format = caps
            .formats
            .iter()
            .copied()
            .find(|f| f.is_srgb())
            .unwrap_or(caps.formats[0]);

        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format,
            width: size.width,
            height: size.height,
            present_mode: wgpu::PresentMode::Fifo,
            alpha_mode: caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };
        surface.configure(&device, &config);

        Ok(Renderer {
            surface,
            device,
            queue,
            config,
            size,
        })
    }

    /// The current surface size in physical pixels.
    pub fn size(&self) -> winit::dpi::PhysicalSize<u32> {
        self.size
    }

    /// Reconfigure the surface after the window changed size. A zero dimension
    /// is ignored (some platforms report it while minimized).
    pub fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        if new_size.width == 0 || new_size.height == 0 {
            return;
        }
        self.size = new_size;
        self.config.width = new_size.width;
        self.config.height = new_size.height;
        self.surface.configure(&self.device, &self.config);
    }

    /// Reconfigure using the last known size — used to recover from a lost or
    /// outdated surface.
    fn reconfigure(&mut self) {
        self.surface.configure(&self.device, &self.config);
    }

    /// Render one frame: for now, clear the whole surface to `clear`.
    pub fn render(&mut self, clear: Color) -> Result<(), RendererError> {
        let frame = match self.surface.get_current_texture() {
            Ok(frame) => frame,
            // The surface fell out of sync with the window (resize, minimize,
            // device change). Reconfigure and skip this frame; the next redraw
            // will paint.
            Err(wgpu::SurfaceError::Outdated | wgpu::SurfaceError::Lost) => {
                self.reconfigure();
                return Ok(());
            }
            Err(wgpu::SurfaceError::OutOfMemory) => {
                return Err(RendererError::OutOfMemory);
            }
            Err(e) => {
                log::warn!("dropped frame: {e:?}");
                return Ok(());
            }
        };

        let view = frame
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("baseui-frame"),
            });

        let [r, g, b, a] = clear.to_linear();
        {
            let _pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("baseui-clear"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    depth_slice: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: r as f64,
                            g: g as f64,
                            b: b as f64,
                            a: a as f64,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            // Future milestones record draw calls into `_pass` here.
        }

        self.queue.submit(std::iter::once(encoder.finish()));
        frame.present();
        Ok(())
    }
}

/// Errors that can occur while creating or driving the renderer.
#[derive(Debug)]
pub enum RendererError {
    Surface(String),
    NoAdapter(String),
    NoDevice(String),
    OutOfMemory,
}

impl std::fmt::Display for RendererError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RendererError::Surface(e) => write!(f, "failed to create surface: {e}"),
            RendererError::NoAdapter(e) => write!(f, "no suitable GPU adapter: {e}"),
            RendererError::NoDevice(e) => write!(f, "failed to create GPU device: {e}"),
            RendererError::OutOfMemory => write!(f, "GPU out of memory"),
        }
    }
}

impl std::error::Error for RendererError {}
