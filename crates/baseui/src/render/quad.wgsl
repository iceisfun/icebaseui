// BaseUI quad shader.
//
// One instanced pipeline draws every 2D primitive as a quad:
//   mode 0 = a rounded, optionally bordered rectangle, anti-aliased via a
//            signed-distance field.
//   mode 1 = a glyph, sampled (as coverage) from the R8 font atlas.
//
// Positions arrive in logical pixels (top-left origin) and are converted to
// physical pixels (× scale) and then to normalized device coordinates. Colors
// arrive already in linear space; the sRGB surface encodes on store.

struct Globals {
    screen: vec2<f32>, // surface size in physical pixels
    scale: f32,        // logical -> physical pixel factor
    _pad: f32,
};

@group(0) @binding(0) var<uniform> globals: Globals;
@group(0) @binding(1) var atlas_tex: texture_2d<f32>;
@group(0) @binding(2) var atlas_samp: sampler;

struct Instance {
    @location(0) rect: vec4<f32>,         // x, y, w, h  (logical px)
    @location(1) uv: vec4<f32>,           // u0, v0, u1, v1  (atlas, glyphs only)
    @location(2) color: vec4<f32>,        // linear straight-alpha
    @location(3) border_color: vec4<f32>, // linear straight-alpha
    @location(4) clip: vec4<f32>,         // x, y, w, h  (logical px)
    @location(5) params: vec4<f32>,       // corner_radius, border_width, mode, _
};

struct VsOut {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) pos_px: vec2<f32>,
    @location(1) uv: vec2<f32>,
    @location(2) color: vec4<f32>,
    @location(3) border_color: vec4<f32>,
    @location(4) rect: vec4<f32>,
    @location(5) clip: vec4<f32>,
    @location(6) params: vec4<f32>,
};

const CORNERS = array<vec2<f32>, 6>(
    vec2<f32>(0.0, 0.0), vec2<f32>(1.0, 0.0), vec2<f32>(0.0, 1.0),
    vec2<f32>(0.0, 1.0), vec2<f32>(1.0, 0.0), vec2<f32>(1.0, 1.0),
);

@vertex
fn vs_main(@builtin(vertex_index) vi: u32, inst: Instance) -> VsOut {
    let corner = CORNERS[vi];
    let pos_px = inst.rect.xy + corner * inst.rect.zw;
    let phys = pos_px * globals.scale;
    let ndc = vec2<f32>(
        phys.x / globals.screen.x * 2.0 - 1.0,
        1.0 - phys.y / globals.screen.y * 2.0,
    );

    var out: VsOut;
    out.clip_position = vec4<f32>(ndc, 0.0, 1.0);
    out.pos_px = pos_px;
    out.uv = mix(inst.uv.xy, inst.uv.zw, corner);
    out.color = inst.color;
    out.border_color = inst.border_color;
    out.rect = inst.rect;
    out.clip = inst.clip;
    out.params = inst.params;
    return out;
}

// Signed distance to a rounded box centered at the origin.
fn sd_round_box(p: vec2<f32>, half: vec2<f32>, r: f32) -> f32 {
    let q = abs(p) - half + vec2<f32>(r);
    return min(max(q.x, q.y), 0.0) + length(max(q, vec2<f32>(0.0))) - r;
}

@fragment
fn fs_main(in: VsOut) -> @location(0) vec4<f32> {
    // Rectangular clip test.
    if (in.pos_px.x < in.clip.x || in.pos_px.y < in.clip.y ||
        in.pos_px.x > in.clip.x + in.clip.z ||
        in.pos_px.y > in.clip.y + in.clip.w) {
        discard;
    }

    // Sample the atlas unconditionally (LOD 0, no derivatives) so the call
    // stays in uniform control flow.
    let glyph_a = textureSampleLevel(atlas_tex, atlas_samp, in.uv, 0.0).r;

    // Rounded-rect coverage — computed at top level so fwidth() is uniform.
    let center = in.rect.xy + in.rect.zw * 0.5;
    let half = in.rect.zw * 0.5;
    let radius = min(in.params.x, min(half.x, half.y));
    let p = in.pos_px - center;
    let d = sd_round_box(p, half, radius);
    let aa = max(fwidth(d), 1e-4);
    let cover = clamp(0.5 - d / aa, 0.0, 1.0);

    let mode = in.params.z;
    if (mode > 0.5) {
        // Glyph: coverage from the atlas, tinted by color.
        return vec4<f32>(in.color.rgb, in.color.a * glyph_a);
    }

    // Shape: blend fill toward border color within the border ring.
    var col = in.color;
    let bw = in.params.y;
    if (bw > 0.0) {
        let bf = clamp(0.5 + (d + bw) / aa, 0.0, 1.0);
        col = vec4<f32>(
            mix(in.color.rgb, in.border_color.rgb, bf),
            mix(in.color.a, in.border_color.a, bf),
        );
    }
    return vec4<f32>(col.rgb, col.a * cover);
}
