//! Text rendering: a font, a glyph atlas, and simple left-to-right layout.
//!
//! A single default sans-serif face is located via `fontdb` (system fonts) and
//! rasterized on demand with `ab_glyph`. Each unique (glyph, pixel-size) pair is
//! rasterized once into a shared R8 coverage atlas using a lightweight shelf
//! packer, then emitted as textured quads (`mode = glyph`) through the same
//! [`QuadPipeline`](super::quad::QuadPipeline) that draws shapes.
//!
//! Layout here is deliberately minimal for the foundation: per-glyph advance
//! widths and `\n` handling, no shaping/kerning/bidi. That is enough for Latin
//! UI labels and can be swapped for a shaping engine (e.g. cosmic-text) later
//! without changing the public paint API.

use std::collections::HashMap;

use ab_glyph::{Font, FontVec, PxScale, ScaleFont};
use baseui_core::Rect;
use baseui_core::paint::TextShape;

use super::quad::{MODE_GLYPH, QuadInstance};

/// Side length of the (square) glyph atlas texture, in texels.
const ATLAS_SIZE: u32 = 1024;

/// Cache key: font glyph id plus the integer pixel size it was rasterized at.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
struct GlyphKey {
    glyph: u16,
    px: u32,
}

/// A rasterized glyph's location in the atlas and its placement metrics, all in
/// physical pixels.
#[derive(Clone, Copy)]
struct AtlasEntry {
    /// Atlas UV rect: u0, v0, u1, v1.
    uv: [f32; 4],
    /// Bitmap size in physical px.
    width: f32,
    height: f32,
    /// Offset from the pen origin (on the baseline) to the bitmap's top-left,
    /// in physical px. `top` is typically negative (above the baseline).
    left: f32,
    top: f32,
}

/// A row-based ("shelf") atlas allocator with 1px padding between glyphs.
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

    /// Reserve a `w × h` region, returning its top-left origin, or `None` if the
    /// atlas is full.
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

/// Owns the font, atlas texture, and glyph cache.
pub struct TextRenderer {
    font: Option<FontVec>,
    atlas_tex: wgpu::Texture,
    atlas_view: wgpu::TextureView,
    atlas_sampler: wgpu::Sampler,
    packer: ShelfPacker,
    /// `None` value = a glyph with no outline (e.g. space); still cached so we
    /// don't retry rasterizing it.
    cache: HashMap<GlyphKey, Option<AtlasEntry>>,
}

impl TextRenderer {
    pub fn new(device: &wgpu::Device) -> Self {
        let font = load_default_font();
        if font.is_none() {
            log::error!("no system sans-serif font found; text will not render");
        }

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

        TextRenderer {
            font,
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
    /// rasterizing any not-yet-cached glyphs into the atlas.
    ///
    /// `scale` is the logical→physical factor: glyphs are rasterized at
    /// `size × scale` physical pixels for crispness, but emitted quads are in
    /// logical pixels.
    pub fn push_text(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        scale: f32,
        shape: &TextShape,
        clip: Rect,
        out: &mut Vec<QuadInstance>,
    ) {
        let Some(font) = self.font.as_ref() else {
            return;
        };

        let px = (shape.size * scale).round().max(1.0);
        let px_scale = PxScale::from(px);
        let scaled = font.as_scaled(px_scale);

        let ascent = scaled.ascent();
        let line_advance = scaled.height() + scaled.line_gap();

        let color = shape.color.to_linear();
        let clip_arr = [clip.left(), clip.top(), clip.width(), clip.height()];

        let mut pen_x = shape.pos.x; // logical px
        let mut baseline = shape.pos.y + ascent / scale; // logical px

        for ch in shape.text.chars() {
            if ch == '\n' {
                pen_x = shape.pos.x;
                baseline += line_advance / scale;
                continue;
            }

            let glyph_id = font.glyph_id(ch);
            let key = GlyphKey {
                glyph: glyph_id.0,
                px: px as u32,
            };

            // Rasterize into the atlas on first sight.
            if !self.cache.contains_key(&key) {
                let entry = rasterize_glyph(font, glyph_id, px_scale, &mut self.packer, |origin, w, h, data| {
                    write_atlas(queue, &self.atlas_tex, origin, w, h, data);
                });
                self.cache.insert(key, entry);
            }

            if let Some(entry) = self.cache.get(&key).copied().flatten() {
                let x = pen_x + entry.left / scale;
                let y = baseline + entry.top / scale;
                let w = entry.width / scale;
                let h = entry.height / scale;
                out.push(QuadInstance {
                    rect: [x, y, w, h],
                    uv: entry.uv,
                    color,
                    border_color: [0.0; 4],
                    clip: clip_arr,
                    params: [0.0, 0.0, MODE_GLYPH, 0.0],
                });
            }

            pen_x += scaled.h_advance(glyph_id) / scale;
        }

        // `device` is currently unused (atlas is fixed-size), but kept in the
        // signature so a future dynamic/multi-page atlas can grow it.
        let _ = device;
    }
}

/// Rasterize `glyph_id` at `px_scale`, allocate an atlas slot, hand the bitmap
/// to `upload`, and return its placement entry. Returns `None` for glyphs with
/// no outline (whitespace) or when the atlas is full.
fn rasterize_glyph(
    font: &FontVec,
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

/// Upload one glyph bitmap into the atlas at `origin`.
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

/// Locate a system sans-serif face and load it as an `ab_glyph` font.
fn load_default_font() -> Option<FontVec> {
    let mut db = fontdb::Database::new();
    db.load_system_fonts();

    // Prefer the configured generic sans-serif; fall back to any face.
    let query = fontdb::Query {
        families: &[fontdb::Family::SansSerif],
        ..Default::default()
    };
    let id = db
        .query(&query)
        .or_else(|| db.faces().next().map(|f| f.id))?;

    db.with_face_data(id, |data, index| {
        FontVec::try_from_vec_and_index(data.to_vec(), index).ok()
    })
    .flatten()
}
