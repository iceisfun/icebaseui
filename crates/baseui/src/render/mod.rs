//! The wgpu rendering backend.
//!
//! The renderer owns all long-lived GPU objects (instance, adapter, device,
//! queue, surface) plus the [`QuadPipeline`] and the [`TextRenderer`]. Each
//! frame it walks a [`Scene`] display list, flattens it into a batch of
//! [`QuadInstance`]s (resolving the nested clip stack and rasterizing any needed
//! glyphs into the font atlas), and draws them in a single instanced pass.

mod glyph;
mod quad;

use std::rc::Rc;
use std::sync::Arc;

use baseui_core::paint::{Command, Primitive, RectShape, Scene};
use baseui_core::{Color, Rect};
use winit::window::Window;

use crate::text::Fonts;
use glyph::GlyphRenderer;
use quad::{MODE_SHAPE, QuadInstance, QuadPipeline};

/// Owns the GPU device, the window surface, and the drawing pipelines.
pub struct Renderer {
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    size: winit::dpi::PhysicalSize<u32>,
    /// Logical -> physical pixel scale factor.
    scale: f32,

    quad: QuadPipeline,
    glyphs: GlyphRenderer,

    /// Reused per-frame instance scratch buffer.
    instances: Vec<QuadInstance>,
}

impl Renderer {
    /// Create a renderer for `window`, sharing the already-loaded [`Fonts`].
    /// Blocks on GPU adapter/device acquisition.
    pub fn new(window: Arc<Window>, fonts: Rc<Fonts>) -> Result<Self, RendererError> {
        pollster::block_on(Self::new_async(window, fonts))
    }

    async fn new_async(window: Arc<Window>, fonts: Rc<Fonts>) -> Result<Self, RendererError> {
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

        let glyphs = GlyphRenderer::new(&device, fonts);
        let quad = QuadPipeline::new(&device, format, glyphs.atlas_view(), glyphs.atlas_sampler());

        Ok(Renderer {
            surface,
            device,
            queue,
            config,
            size,
            scale,
            quad,
            glyphs,
            instances: Vec::new(),
        })
    }

    /// The current surface size in physical pixels.
    pub fn size(&self) -> winit::dpi::PhysicalSize<u32> {
        self.size
    }

    /// The current logical->physical scale factor.
    pub fn scale_factor(&self) -> f32 {
        self.scale
    }

    /// Update the logical->physical scale factor (window moved to another
    /// monitor / DPI changed).
    pub fn set_scale_factor(&mut self, scale: f64) {
        self.scale = scale as f32;
    }

    /// Reconfigure the surface after a resize. Zero dimensions are ignored.
    pub fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        if new_size.width == 0 || new_size.height == 0 {
            return;
        }
        self.size = new_size;
        self.config.width = new_size.width;
        self.config.height = new_size.height;
        self.surface.configure(&self.device, &self.config);
    }

    fn reconfigure(&mut self) {
        self.surface.configure(&self.device, &self.config);
    }

    /// The surface size in logical pixels.
    pub fn logical_size(&self) -> baseui_core::Size {
        baseui_core::Size::new(
            self.size.width as f32 / self.scale,
            self.size.height as f32 / self.scale,
        )
    }

    /// Flatten `scene` into `self.instances`, resolving the clip stack and
    /// rasterizing glyphs into the atlas as needed.
    fn build_instances(&mut self, scene: &Scene) {
        self.instances.clear();

        let logical = self.logical_size();
        let root_clip = Rect::from_xywh(0.0, 0.0, logical.width, logical.height);
        let mut clip_stack: Vec<Rect> = vec![root_clip];

        for command in scene.commands() {
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
                            let Renderer {
                                glyphs,
                                instances,
                                queue,
                                scale,
                                ..
                            } = self;
                            glyphs.push_text(queue, *scale, shape, clip, instances);
                        }
                    }
                }
            }
        }
    }

    /// Render one frame from `scene`, clearing to `clear` first.
    pub fn render(&mut self, scene: &Scene, clear: Color) -> Result<(), RendererError> {
        self.build_instances(scene);

        let screen = [self.size.width as f32, self.size.height as f32];
        // Take the scratch buffer out so we can borrow the pipeline mutably
        // without aliasing `self.instances`.
        let instances = std::mem::take(&mut self.instances);
        self.quad
            .prepare(&self.device, &self.queue, screen, self.scale, &instances);

        let frame = match self.surface.get_current_texture() {
            Ok(frame) => frame,
            Err(wgpu::SurfaceError::Outdated | wgpu::SurfaceError::Lost) => {
                self.reconfigure();
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

        // Return the scratch buffer for reuse next frame.
        self.instances = instances;
        Ok(())
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
