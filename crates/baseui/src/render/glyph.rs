//! GPU glyph rasterization and the shared font atlas.
//!
//! Rasterizes each unique (font, glyph, pixel-size) once into a shelf-packed R8
//! coverage atlas and emits textured quads (`mode = glyph`) through the
//! [`QuadPipeline`](super::quad::QuadPipeline). Fonts and measurement live in
//! [`crate::text`]; this module borrows a shared [`Fonts`] and only owns GPU
//! state (the atlas texture) plus the rasterized-glyph cache.
//!
//! Layout is minimal LTR (advance widths + `\n`); see `docs/rich-text.md` for
//! the planned styled-run / cached-layout upgrade.

use std::collections::HashMap;
use std::rc::Rc;

use ab_glyph::{Font, PxScale, ScaleFont};
use baseui_core::Rect;
use baseui_core::paint::TextShape;

use super::quad::{MODE_GLYPH, QuadInstance};
use crate::text::{FontId, Fonts};

const ATLAS_SIZE: u32 = 1024;

/// Cache key: font family, glyph id, and integer rasterization pixel size.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
struct GlyphKey {
    font: FontId,
    glyph: u16,
    px: u32,
}

/// A rasterized glyph's atlas location and placement metrics (physical px).
#[derive(Clone, Copy)]
struct AtlasEntry {
    uv: [f32; 4],
    width: f32,
    height: f32,
    left: f32,
    top: f32,
}

struct ShelfPacker {
    x: u32,
    y: u32,
    shelf_height: u32,
}

impl ShelfPacker {
    fn new() -> Self {
        ShelfPacker {
            x: 0,
            y: 0,
            shelf_height: 0,
        }
    }

    fn alloc(&mut self, w: u32, h: u32) -> Option<(u32, u32)> {
        if w > ATLAS_SIZE || h > ATLAS_SIZE {
            return None;
        }
        if self.x + w > ATLAS_SIZE {
            self.x = 0;
            self.y += self.shelf_height + 1;
            self.shelf_height = 0;
        }
        if self.y + h > ATLAS_SIZE {
            return None;
        }
        let pos = (self.x, self.y);
        self.x += w + 1;
        self.shelf_height = self.shelf_height.max(h);
        Some(pos)
    }
}

/// Owns the atlas texture and glyph cache; borrows shared [`Fonts`].
pub struct GlyphRenderer {
    fonts: Rc<Fonts>,
    atlas_tex: wgpu::Texture,
    atlas_view: wgpu::TextureView,
    atlas_sampler: wgpu::Sampler,
    packer: ShelfPacker,
    cache: HashMap<GlyphKey, Option<AtlasEntry>>,
}

impl GlyphRenderer {
    pub fn new(device: &wgpu::Device, fonts: Rc<Fonts>) -> Self {
        let atlas_tex = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("baseui-glyph-atlas"),
            size: wgpu::Extent3d {
                width: ATLAS_SIZE,
                height: ATLAS_SIZE,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::R8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        let atlas_view = atlas_tex.create_view(&wgpu::TextureViewDescriptor::default());
        let atlas_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("baseui-glyph-sampler"),
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Nearest,
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            ..Default::default()
        });

        GlyphRenderer {
            fonts,
            atlas_tex,
            atlas_view,
            atlas_sampler,
            packer: ShelfPacker::new(),
            cache: HashMap::new(),
        }
    }

    pub fn atlas_view(&self) -> &wgpu::TextureView {
        &self.atlas_view
    }

    pub fn atlas_sampler(&self) -> &wgpu::Sampler {
        &self.atlas_sampler
    }

    /// Lay out `shape` and append one glyph quad per visible glyph to `out`,
    /// rasterizing not-yet-cached glyphs into the atlas at `size × scale` px.
    pub fn push_text(
        &mut self,
        queue: &wgpu::Queue,
        scale: f32,
        shape: &TextShape,
        clip: Rect,
        out: &mut Vec<QuadInstance>,
    ) {
        let font_id = shape.font;
        let Some(font) = self.fonts.face(font_id) else {
            return; // font (e.g. an unregistered icon font) not loaded
        };

        // The global text scale must be applied here exactly as it is in
        // `Fonts` measurement, or the caret/hit-testing would drift from what is
        // actually drawn. `scale` is the DPI (logical -> physical) factor.
        let logical_size = shape.size * crate::text::scale();
        let px = (logical_size * scale).round().max(1.0);
        let px_scale = PxScale::from(px);
        let scaled = font.as_scaled(px_scale);
        let ascent = scaled.ascent();
        let line_advance = scaled.height() + scaled.line_gap();

        let color = shape.color.to_linear();
        let clip_arr = [clip.left(), clip.top(), clip.width(), clip.height()];

        let mut pen_x = shape.pos.x;
        let mut baseline = shape.pos.y + ascent / scale;

        for ch in shape.text.chars() {
            if ch == '\n' {
                pen_x = shape.pos.x;
                baseline += line_advance / scale;
                continue;
            }

            let glyph_id = font.glyph_id(ch);
            let key = GlyphKey {
                font: font_id,
                glyph: glyph_id.0,
                px: px as u32,
            };

            if !self.cache.contains_key(&key) {
                let entry = rasterize_glyph(
                    font,
                    glyph_id,
                    px_scale,
                    &mut self.packer,
                    |origin, w, h, data| write_atlas(queue, &self.atlas_tex, origin, w, h, data),
                );
                self.cache.insert(key, entry);
            }

            if let Some(entry) = self.cache.get(&key).copied().flatten() {
                out.push(QuadInstance {
                    rect: [
                        pen_x + entry.left / scale,
                        baseline + entry.top / scale,
                        entry.width / scale,
                        entry.height / scale,
                    ],
                    uv: entry.uv,
                    color,
                    border_color: [0.0; 4],
                    clip: clip_arr,
                    params: [0.0, 0.0, MODE_GLYPH, 0.0],
                });
            }

            pen_x += scaled.h_advance(glyph_id) / scale;
        }
    }
}

fn rasterize_glyph(
    font: &ab_glyph::FontVec,
    glyph_id: ab_glyph::GlyphId,
    px_scale: PxScale,
    packer: &mut ShelfPacker,
    upload: impl FnOnce((u32, u32), u32, u32, &[u8]),
) -> Option<AtlasEntry> {
    let glyph = glyph_id.with_scale(px_scale);
    let outlined = font.outline_glyph(glyph)?;
    let bounds = outlined.px_bounds();
    let w = bounds.width().ceil() as u32;
    let h = bounds.height().ceil() as u32;
    if w == 0 || h == 0 {
        return None;
    }

    let (ox, oy) = packer.alloc(w, h)?;

    let mut bitmap = vec![0u8; (w * h) as usize];
    outlined.draw(|gx, gy, coverage| {
        if gx < w && gy < h {
            bitmap[(gy * w + gx) as usize] = (coverage * 255.0 + 0.5) as u8;
        }
    });
    upload((ox, oy), w, h, &bitmap);

    let inv = 1.0 / ATLAS_SIZE as f32;
    Some(AtlasEntry {
        uv: [
            ox as f32 * inv,
            oy as f32 * inv,
            (ox + w) as f32 * inv,
            (oy + h) as f32 * inv,
        ],
        width: w as f32,
        height: h as f32,
        left: bounds.min.x,
        top: bounds.min.y,
    })
}

fn write_atlas(
    queue: &wgpu::Queue,
    atlas: &wgpu::Texture,
    origin: (u32, u32),
    w: u32,
    h: u32,
    data: &[u8],
) {
    queue.write_texture(
        wgpu::TexelCopyTextureInfo {
            texture: atlas,
            mip_level: 0,
            origin: wgpu::Origin3d {
                x: origin.0,
                y: origin.1,
                z: 0,
            },
            aspect: wgpu::TextureAspect::All,
        },
        data,
        wgpu::TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some(w),
            rows_per_image: Some(h),
        },
        wgpu::Extent3d {
            width: w,
            height: h,
            depth_or_array_layers: 1,
        },
    );
}
