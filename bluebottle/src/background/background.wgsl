// Background shader for the main screen.
//
// Two pipelines share this module:
//   * the separable Gaussian blur (`vs_fullscreen` + `fs_blur`), run twice as a
//     pre-pass over the spotlight image, and
//   * the composite (`vs_fullscreen` + `fs_composite`), which lays the blurred
//     poster (or a procedural gradient) under a dark vertical tint.
//
// All colour maths is in linear space: the source image and intermediate
// targets are sRGB textures (so sampling decodes and rendering re-encodes), and
// the constant colours arrive as linear uniforms.

struct VsOut {
    @builtin(position) position: vec4<f32>,
    // `uv` spans the widget rect, with (0, 0) at the top-left.
    @location(0) uv: vec2<f32>,
}

// A full-screen triangle; the viewport clips it to the widget rect.
@vertex
fn vs_fullscreen(@builtin(vertex_index) index: u32) -> VsOut {
    var corners = array<vec2<f32>, 3>(
        vec2<f32>(-1.0, -1.0),
        vec2<f32>(3.0, -1.0),
        vec2<f32>(-1.0, 3.0),
    );
    let xy = corners[index];

    var out: VsOut;
    out.position = vec4<f32>(xy, 0.0, 1.0);
    out.uv = vec2<f32>((xy.x + 1.0) * 0.5, (1.0 - xy.y) * 0.5);
    return out;
}

// ---------------------------------------------------------------------------
// Separable Gaussian blur pre-pass.
// ---------------------------------------------------------------------------

struct Blur {
    // Size of one source texel, `1.0 / source_size`.
    texel: vec2<f32>,
    // `(1, 0)` for the horizontal pass, `(0, 1)` for the vertical pass.
    direction: vec2<f32>,
    // Blur radius, in source pixels.
    radius: f32,
    // Scalar padding to a 32-byte buffer; a `vec3` here would force 16-byte
    // alignment and bloat the struct to 48 bytes.
    _pad0: f32,
    _pad1: f32,
    _pad2: f32,
}

@group(0) @binding(0) var<uniform> blur: Blur;
@group(0) @binding(1) var blur_source: texture_2d<f32>;
@group(0) @binding(2) var blur_sampler: sampler;

const TAPS: i32 = 12;

@fragment
fn fs_blur(in: VsOut) -> @location(0) vec4<f32> {
    let radius = max(blur.radius, 0.001);
    let sigma = radius / 3.0;
    let step = radius / f32(TAPS);

    var sum = vec3<f32>(0.0);
    var weight_total = 0.0;
    for (var i = -TAPS; i <= TAPS; i = i + 1) {
        let distance = f32(i) * step;
        let weight = exp(-(distance * distance) / (2.0 * sigma * sigma));
        let offset = blur.direction * (distance * blur.texel);
        sum += textureSampleLevel(blur_source, blur_sampler, in.uv + offset, 0.0).rgb * weight;
        weight_total += weight;
    }
    return vec4<f32>(sum / weight_total, 1.0);
}

// ---------------------------------------------------------------------------
// Composite pass.
// ---------------------------------------------------------------------------

struct Composite {
    target_size: vec2<f32>,
    source_size: vec2<f32>,
    // Linear base colour the page settles into (app background).
    base_color: vec4<f32>,
    // Linear highlight colour for the procedural gradient's glow.
    highlight: vec4<f32>,
    // Saturation multiplier applied to the poster wash.
    saturate: f32,
    // `1.0` to sample the blurred poster, `0.0` for the procedural gradient.
    mode: f32,
    _pad: vec2<f32>,
}

@group(0) @binding(0) var<uniform> comp: Composite;
@group(0) @binding(1) var poster: texture_2d<f32>;
@group(0) @binding(2) var poster_sampler: sampler;

// Poster overshoot, so the up-scaled blur leaves no sharp edge at the bounds.
const ZOOM: f32 = 1.15;

// Vertical tint alpha: translucent at the top, solid base by the bottom.
fn tint_alpha(y: f32) -> f32 {
    if y < 0.6 {
        return mix(0.55, 0.82, y / 0.6);
    }
    return mix(0.82, 1.0, clamp((y - 0.6) / 0.35, 0.0, 1.0));
}

// Cover-fit the source into the target, then zoom in by `ZOOM`.
fn poster_uv(uv: vec2<f32>) -> vec2<f32> {
    let target_aspect = comp.target_size.x / comp.target_size.y;
    let source_aspect = comp.source_size.x / comp.source_size.y;

    var centered = uv - 0.5;
    if target_aspect > source_aspect {
        centered.y *= source_aspect / target_aspect;
    } else {
        centered.x *= target_aspect / source_aspect;
    }
    return centered / ZOOM + 0.5;
}

@fragment
fn fs_composite(in: VsOut) -> @location(0) vec4<f32> {
    let base = comp.base_color.rgb;

    var content: vec3<f32>;
    if comp.mode > 0.5 {
        var wash = textureSampleLevel(poster, poster_sampler, poster_uv(in.uv), 0.0).rgb;
        let luma = dot(wash, vec3<f32>(0.2126, 0.7152, 0.0722));
        wash = mix(vec3<f32>(luma), wash, comp.saturate);

        // Poster mask: opaque near the top, gone by 80% down, knocked back to 0.55.
        let mask = (1.0 - smoothstep(0.3, 0.8, in.uv.y)) * 0.55;
        content = mix(base, wash, mask);
    } else {
        // Soft highlight near the top-centre, trending into the dark base.
        let center = vec2<f32>(0.5, 0.22);
        var delta = in.uv - center;
        delta.x *= comp.target_size.x / comp.target_size.y;
        let glow = 1.0 - smoothstep(0.0, 0.75, length(delta));
        content = mix(base, comp.highlight.rgb, glow * 0.85);
    }

    let final_color = mix(content, base, tint_alpha(in.uv.y));
    return vec4<f32>(final_color, 1.0);
}
