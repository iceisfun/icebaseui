//! The instanced-quad pipeline: the single GPU program that draws every 2D
//! primitive (rounded rects, borders, and glyphs) as a batched set of quads.
//!
//! See `quad.wgsl` for the shader. The CPU side here owns the pipeline, the
//! globals uniform, and a growable instance buffer, and exposes [`QuadInstance`]
//! (the per-quad payload the renderer fills) plus draw plumbing.

use bytemuck::{Pod, Zeroable};

/// Per-quad instance data. Field order and layout must match the `Instance`
/// struct in `quad.wgsl`.
#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub struct QuadInstance {
    /// x, y, w, h in logical pixels.
    pub rect: [f32; 4],
    /// Atlas UV rect (u0, v0, u1, v1); ignored for shapes.
    pub uv: [f32; 4],
    /// Fill color, linear straight-alpha.
    pub color: [f32; 4],
    /// Border color, linear straight-alpha.
    pub border_color: [f32; 4],
    /// Clip rect x, y, w, h in logical pixels.
    pub clip: [f32; 4],
    /// corner_radius, border_width, mode (0 = shape, 1 = glyph), padding.
    pub params: [f32; 4],
}

/// Instance vertex-buffer mode flags packed into `params.z`.
pub const MODE_SHAPE: f32 = 0.0;
pub const MODE_GLYPH: f32 = 1.0;

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct Globals {
    screen: [f32; 2],
    scale: f32,
    _pad: f32,
}

/// Owns the quad render pipeline, its bind group, and dynamic GPU buffers.
pub struct QuadPipeline {
    pipeline: wgpu::RenderPipeline,
    bind_group: wgpu::BindGroup,
    globals_buf: wgpu::Buffer,
    instance_buf: wgpu::Buffer,
    instance_capacity: usize,
}

impl QuadPipeline {
    /// Build the pipeline. `atlas_view`/`atlas_sampler` are the font atlas the
    /// glyph path samples; they are bound once here (the atlas texture is
    /// updated in place via `write_texture`, so the bind group stays valid).
    pub fn new(
        device: &wgpu::Device,
        surface_format: wgpu::TextureFormat,
        atlas_view: &wgpu::TextureView,
        atlas_sampler: &wgpu::Sampler,
    ) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("baseui-quad-shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("quad.wgsl").into()),
        });

        let globals_buf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("baseui-quad-globals"),
            size: std::mem::size_of::<Globals>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("baseui-quad-bgl"),
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                            view_dimension: wgpu::TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 2,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        count: None,
                    },
                ],
            });

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("baseui-quad-bg"),
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: globals_buf.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(atlas_view),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::Sampler(atlas_sampler),
                },
            ],
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("baseui-quad-pl"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        // One instance stepped per quad; six vertices generated in the shader.
        let instance_layout = wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<QuadInstance>() as u64,
            step_mode: wgpu::VertexStepMode::Instance,
            attributes: &wgpu::vertex_attr_array![
                0 => Float32x4, // rect
                1 => Float32x4, // uv
                2 => Float32x4, // color
                3 => Float32x4, // border_color
                4 => Float32x4, // clip
                5 => Float32x4, // params
            ],
        };

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("baseui-quad-pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[instance_layout],
                compilation_options: Default::default(),
            },
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                ..Default::default()
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: surface_format,
                    blend: Some(wgpu::BlendState {
                        color: wgpu::BlendComponent {
                            src_factor: wgpu::BlendFactor::SrcAlpha,
                            dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
                            operation: wgpu::BlendOperation::Add,
                        },
                        alpha: wgpu::BlendComponent {
                            src_factor: wgpu::BlendFactor::One,
                            dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
                            operation: wgpu::BlendOperation::Add,
                        },
                    }),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            multiview: None,
            cache: None,
        });

        let instance_capacity = 256;
        let instance_buf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("baseui-quad-instances"),
            size: (instance_capacity * std::mem::size_of::<QuadInstance>()) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        QuadPipeline {
            pipeline,
            bind_group,
            globals_buf,
            instance_buf,
            instance_capacity,
        }
    }

    /// Upload globals and instance data for this frame, growing the instance
    /// buffer if needed.
    pub fn prepare(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        screen_physical: [f32; 2],
        scale: f32,
        instances: &[QuadInstance],
    ) {
        queue.write_buffer(
            &self.globals_buf,
            0,
            bytemuck::bytes_of(&Globals {
                screen: screen_physical,
                scale,
                _pad: 0.0,
            }),
        );

        if instances.is_empty() {
            return;
        }

        if instances.len() > self.instance_capacity {
            let new_cap = instances.len().next_power_of_two();
            self.instance_buf = device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("baseui-quad-instances"),
                size: (new_cap * std::mem::size_of::<QuadInstance>()) as u64,
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
            self.instance_capacity = new_cap;
        }

        queue.write_buffer(&self.instance_buf, 0, bytemuck::cast_slice(instances));
    }

    /// Record the draw for `instance_count` instances prepared this frame.
    pub fn draw(&self, pass: &mut wgpu::RenderPass<'_>, instance_count: u32) {
        if instance_count == 0 {
            return;
        }
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &self.bind_group, &[]);
        pass.set_vertex_buffer(0, self.instance_buf.slice(..));
        pass.draw(0..6, 0..instance_count);
    }
}
