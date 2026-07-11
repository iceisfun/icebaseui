//! The wgpu rendering backend.
//!
//! Split in two so that a single GPU device can drive **many windows** (floating
//! panels, tool windows, detached dock tabs):
//!
//! - [`GpuContext`] — shared: instance, adapter, device, queue, the quad
//!   pipeline, and the glyph/icon atlas. One of these for the whole app.
//! - [`WindowRenderer`] — per window: its surface, surface config, physical size,
//!   and DPI scale.
//!
//! Each frame, [`GpuContext::render`] walks a [`Scene`] for one window, flattens
//! it into [`QuadInstance`]s (resolving the clip stack and rasterizing any new
//! glyphs into the shared atlas), and draws them in a single instanced pass.
//!
//! Because the glyph cache keys on the *rasterized pixel size*
//! (`size × text_scale × dpi_scale`), windows living on monitors with different
//! DPI simply produce additional atlas entries — mixed-DPI multi-monitor works
//! without any special handling.

mod glyph;
mod quad;

use std::rc::Rc;
use std::sync::Arc;

use baseui_core::paint::{Command, Primitive, RectShape, Scene};
use baseui_core::{Color, Rect, Size};
use winit::window::Window;

use crate::text::Fonts;
use glyph::GlyphRenderer;
use quad::{MODE_SHAPE, QuadInstance, QuadPipeline};

/// GPU state shared by every window: device, queue, pipeline, and glyph atlas.
pub struct GpuContext {
    instance: wgpu::Instance,
    adapter: wgpu::Adapter,
    device: wgpu::Device,
    queue: wgpu::Queue,
    /// Surface format the pipeline was built for; all windows use it.
    format: wgpu::TextureFormat,
    quad: QuadPipeline,
    glyphs: GlyphRenderer,
    /// Reused per-frame instance scratch buffer.
    instances: Vec<QuadInstance>,
}

/// Per-window rendering state: its surface and geometry.
pub struct WindowRenderer {
    surface: wgpu::Surface<'static>,
    config: wgpu::SurfaceConfiguration,
    /// Physical pixels.
    size: winit::dpi::PhysicalSize<u32>,
    /// Logical -> physical scale factor for *this* window (its monitor's DPI).
    scale: f32,
}

impl WindowRenderer {
    /// The surface size in physical pixels.
    pub fn size(&self) -> winit::dpi::PhysicalSize<u32> {
        self.size
    }

    /// This window's logical->physical scale factor.
    pub fn scale_factor(&self) -> f32 {
        self.scale
    }

    /// Update the scale factor (window moved to a different-DPI monitor).
    pub fn set_scale_factor(&mut self, scale: f64) {
        self.scale = scale as f32;
    }

    /// The surface size in logical pixels.
    pub fn logical_size(&self) -> Size {
        Size::new(
            self.size.width as f32 / self.scale,
            self.size.height as f32 / self.scale,
        )
    }

    /// Reconfigure after a resize. Zero dimensions are ignored (minimized).
    pub fn resize(&mut self, gpu: &GpuContext, new_size: winit::dpi::PhysicalSize<u32>) {
        if new_size.width == 0 || new_size.height == 0 {
            return;
        }
        self.size = new_size;
        self.config.width = new_size.width;
        self.config.height = new_size.height;
        self.surface.configure(&gpu.device, &self.config);
    }

    fn reconfigure(&mut self, device: &wgpu::Device) {
        self.surface.configure(device, &self.config);
    }
}

impl GpuContext {
    /// Create the shared GPU context together with the renderer for the first
    /// window. Blocks on adapter/device acquisition.
    pub fn new(
        window: Arc<Window>,
        fonts: Rc<Fonts>,
    ) -> Result<(GpuContext, WindowRenderer), RendererError> {
        pollster::block_on(Self::new_async(window, fonts))
    }

    async fn new_async(
        window: Arc<Window>,
        fonts: Rc<Fonts>,
    ) -> Result<(GpuContext, WindowRenderer), RendererError> {
        let mut size = window.inner_size();
        size.width = size.width.max(1);
        size.height = size.height.max(1);
        let scale = window.scale_factor() as f32;

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
        let format = caps
            .formats
            .iter()
            .copied()
            .find(|f| f.is_srgb())
            .unwrap_or(caps.formats[0]);

        let config = surface_config(format, size, &caps);
        surface.configure(&device, &config);

        let glyphs = GlyphRenderer::new(&device, fonts);
        let quad = QuadPipeline::new(&device, format, glyphs.atlas_view(), glyphs.atlas_sampler());

        let gpu = GpuContext {
            instance,
            adapter,
            device,
            queue,
            format,
            quad,
            glyphs,
            instances: Vec::new(),
        };
        let window_renderer = WindowRenderer {
            surface,
            config,
            size,
            scale,
        };
        Ok((gpu, window_renderer))
    }

    /// Create a renderer for an **additional** window on the same device — the
    /// mechanism behind floating panels and detached dock tabs.
    pub fn add_window(&self, window: Arc<Window>) -> Result<WindowRenderer, RendererError> {
        let mut size = window.inner_size();
        size.width = size.width.max(1);
        size.height = size.height.max(1);
        let scale = window.scale_factor() as f32;

        let surface = self
            .instance
            .create_surface(window.clone())
            .map_err(|e| RendererError::Surface(e.to_string()))?;

        let caps = surface.get_capabilities(&self.adapter);
        if !caps.formats.contains(&self.format) {
            // The pipeline is built for one format; a surface that cannot offer
            // it would render incorrectly.
            log::warn!(
                "new window does not support the pipeline surface format {:?}; supported: {:?}",
                self.format,
                caps.formats
            );
        }

        let config = surface_config(self.format, size, &caps);
        surface.configure(&self.device, &config);

        Ok(WindowRenderer {
            surface,
            config,
            size,
            scale,
        })
    }

    /// Flatten `scene` into instances for `window` (base layer, then the overlay
    /// layer on top; each layer gets its own clip stack so popups escape clips).
    fn build_instances(&mut self, window: &WindowRenderer, scene: &Scene) {
        self.instances.clear();
        self.flatten_commands(window, scene.commands());
        self.flatten_commands(window, scene.overlay());
    }

    fn flatten_commands(&mut self, window: &WindowRenderer, commands: &[Command]) {
        let logical = window.logical_size();
        let root_clip = Rect::from_xywh(0.0, 0.0, logical.width, logical.height);
        let mut clip_stack: Vec<Rect> = vec![root_clip];

        for command in commands {
            match command {
                Command::PushClip(rect) => {
                    let current = *clip_stack.last().unwrap();
                    clip_stack.push(current.intersect(*rect));
                }
                Command::PopClip => {
                    if clip_stack.len() > 1 {
                        clip_stack.pop();
                    }
                }
                Command::Draw(primitive) => {
                    let clip = *clip_stack.last().unwrap();
                    match primitive {
                        Primitive::Rect(shape) => {
                            self.instances.push(rect_instance(shape, clip));
                        }
                        Primitive::Text(shape) => {
                            // Split the borrow: the glyph renderer needs &mut
                            // atlas state and pushes into the instance buffer.
                            let GpuContext {
                                glyphs,
                                instances,
                                queue,
                                ..
                            } = self;
                            glyphs.push_text(queue, window.scale, shape, clip, instances);
                        }
                    }
                }
            }
        }
    }

    /// Render one frame of `scene` into `window`, clearing to `clear` first.
    ///
    /// Windows are rendered one at a time, each with its own submit, so the
    /// shared instance buffer is safe: transfers and draws execute in submission
    /// order.
    pub fn render(
        &mut self,
        window: &mut WindowRenderer,
        scene: &Scene,
        clear: Color,
    ) -> Result<(), RendererError> {
        self.build_instances(window, scene);

        let screen = [window.size.width as f32, window.size.height as f32];
        let instances = std::mem::take(&mut self.instances);
        self.quad
            .prepare(&self.device, &self.queue, screen, window.scale, &instances);

        let frame = match window.surface.get_current_texture() {
            Ok(frame) => frame,
            Err(wgpu::SurfaceError::Outdated | wgpu::SurfaceError::Lost) => {
                window.reconfigure(&self.device);
                self.instances = instances;
                return Ok(());
            }
            Err(wgpu::SurfaceError::OutOfMemory) => return Err(RendererError::OutOfMemory),
            Err(e) => {
                log::warn!("dropped frame: {e:?}");
                self.instances = instances;
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
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("baseui-main"),
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
            self.quad.draw(&mut pass, instances.len() as u32);
        }

        self.queue.submit(std::iter::once(encoder.finish()));
        frame.present();

        self.instances = instances;
        Ok(())
    }
}

fn surface_config(
    format: wgpu::TextureFormat,
    size: winit::dpi::PhysicalSize<u32>,
    caps: &wgpu::SurfaceCapabilities,
) -> wgpu::SurfaceConfiguration {
    wgpu::SurfaceConfiguration {
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        format,
        width: size.width,
        height: size.height,
        present_mode: wgpu::PresentMode::Fifo,
        alpha_mode: caps.alpha_modes[0],
        view_formats: vec![],
        desired_maximum_frame_latency: 2,
    }
}

/// Build a shape instance for a rectangle under the given clip.
fn rect_instance(shape: &RectShape, clip: Rect) -> QuadInstance {
    QuadInstance {
        rect: [
            shape.rect.left(),
            shape.rect.top(),
            shape.rect.width(),
            shape.rect.height(),
        ],
        uv: [0.0; 4],
        color: shape.fill.to_linear(),
        border_color: shape.border_color.to_linear(),
        clip: [clip.left(), clip.top(), clip.width(), clip.height()],
        params: [shape.corner_radius, shape.border_width, MODE_SHAPE, 0.0],
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
