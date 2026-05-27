// Background shader for the main screen. Two pipelines share this module: a
// separable Gaussian blur (`vs_fullscreen` + `fs_blur`) run twice over the
// backdrop image, then a composite (`vs_fullscreen` + `fs_composite`) that lays
// the blurred poster (or a procedural gradient) under a dark vertical tint.
//
// Colour maths runs in linear space: source and intermediate targets are sRGB
// textures, and the constant colours arrive as linear uniforms.

struct VsOut {
    @builtin(position) position: vec4<f32>,
    // Spans the widget rect, with (0, 0) at the top-left.
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

struct Blur {
    // Size of one source texel, `1.0 / source_size`.
    texel: vec2<f32>,
    // `(1, 0)` horizontal pass, `(0, 1)` vertical pass.
    direction: vec2<f32>,
    // Blur radius, in source pixels.
    radius: f32,
    // Padding to 32 bytes; scalars avoid the 16-byte alignment a `vec3` forces.
    _pad0: f32,
    _pad1: f32,
    _pad2: f32,
}

@group(0) @binding(0) var<uniform> blur: Blur;
@group(0) @binding(1) var blur_source: texture_2d<f32>;
@group(0) @binding(2) var blur_sampler: sampler;

// Cap on taps per side, keeping the one-time cost finite at large radii. Below
// it the tap count tracks the radius for ~1-texel spacing.
const MAX_TAPS: i32 = 64;

@fragment
fn fs_blur(in: VsOut) -> @location(0) vec4<f32> {
    let radius = max(blur.radius, 0.001);
    let sigma = radius / 3.0;
    // Sample out to ~4.5σ so the Gaussian tail isn't clipped, at ~one tap per
    // source texel — coarser spacing reads as stepping once up-scaled.
    let extent = radius * 1.5;
    let taps = min(i32(ceil(extent)), MAX_TAPS);
    let step = extent / f32(max(taps, 1));

    var sum = vec3<f32>(0.0);
    var weight_total = 0.0;
    for (var i = -taps; i <= taps; i = i + 1) {
        let distance = f32(i) * step;
        let weight = exp(-(distance * distance) / (2.0 * sigma * sigma));
        let offset = blur.direction * (distance * blur.texel);
        sum += textureSampleLevel(blur_source, blur_sampler, in.uv + offset, 0.0).rgb * weight;
        weight_total += weight;
    }
    return vec4<f32>(sum / weight_total, 1.0);
}

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
    // Image wash opacity at the top / at `image_fade`.
    image_opacity_start: f32,
    image_opacity_end: f32,
    // Background-overlay opacity at `bg_start` / at `bg_end`.
    bg_opacity_start: f32,
    bg_opacity_end: f32,
    // Fraction by which the image wash has eased to its end opacity.
    image_fade: f32,
    // Fraction at which the base colour begins coming in over the image.
    bg_start: f32,
    // Fraction by which the soft base-colour fade reaches solid.
    bg_end: f32,
    // Fraction at/below which the background snaps to fully solid (hard edge).
    bg_solid: f32,
    // Vertical focal point for the cover-fit (0 = top, 0.5 = centre).
    focus: f32,
    // Zoom applied to the cover-fit image (1.0 = none).
    zoom: f32,
}

@group(0) @binding(0) var<uniform> comp: Composite;
@group(0) @binding(1) var poster: texture_2d<f32>;
@group(0) @binding(2) var poster_sampler: sampler;

// Cover-fit the source across the full window, centred, then zoom in by `zoom`.
// The image is positioned as if it filled the window; the fade and blur only
// govern how much of it shows.
fn poster_uv(uv: vec2<f32>) -> vec2<f32> {
    let target_aspect = comp.target_size.x / comp.target_size.y;
    let source_aspect = comp.source_size.x / comp.source_size.y;

    // Anchor horizontally at centre, vertically at `focus`, like CSS
    // `background-position: center <focus>` (0 = top, 0.5 = centre).
    let anchor = vec2<f32>(0.5, comp.focus);
    var centered = uv - anchor;
    if target_aspect > source_aspect {
        centered.y *= source_aspect / target_aspect;
    } else {
        centered.x *= target_aspect / source_aspect;
    }
    return centered / max(comp.zoom, 0.01) + anchor;
}

// sRGB transfer functions, used to dither in display space (where the 8-bit
// quantisation happens) rather than in linear space.
fn linear_to_srgb(c: vec3<f32>) -> vec3<f32> {
    let lo = c * 12.92;
    let hi = 1.055 * pow(max(c, vec3<f32>(0.0)), vec3<f32>(1.0 / 2.4)) - 0.055;
    return select(hi, lo, c <= vec3<f32>(0.0031308));
}

fn srgb_to_linear(c: vec3<f32>) -> vec3<f32> {
    let lo = c / 12.92;
    let hi = pow((c + 0.055) / 1.055, vec3<f32>(2.4));
    return select(hi, lo, c <= vec3<f32>(0.04045));
}

// A cheap hash in [0, 1) from a pixel coordinate.
fn hash12(p: vec2<f32>) -> f32 {
    var p3 = fract(vec3<f32>(p.xyx) * 0.1031);
    p3 += dot(p3, p3.yzx + 33.33);
    return fract((p3.x + p3.y) * p3.z);
}

// Triangular-PDF dither in [-1, 1] from two hashes of the pixel coordinate.
fn dither(p: vec2<f32>) -> f32 {
    return hash12(p) + hash12(p + 17.0) - 1.0;
}

@fragment
fn fs_composite(in: VsOut) -> @location(0) vec4<f32> {
    // Composite in sRGB (display) space to match CSS gradient interpolation:
    // blur and saturate run in linear, then everything converts to sRGB for the
    // opacity/coverage mixes and back to linear for output.
    let base = linear_to_srgb(comp.base_color.rgb);

    // The wash at the top: the image knocked back by its opacity so the base
    // shows through, or the procedural highlight, over the base fill.
    var wash_look: vec3<f32>;
    if comp.mode > 0.5 {
        var wash = textureSampleLevel(poster, poster_sampler, poster_uv(in.uv), 0.0).rgb;
        let luma = dot(wash, vec3<f32>(0.2126, 0.7152, 0.0722));
        wash = mix(vec3<f32>(luma), wash, comp.saturate);
        let wash_srgb = linear_to_srgb(wash);

        // Ease opacity from `image_opacity_start` at the top to
        // `image_opacity_end` by `image_fade`.
        let img_t = smoothstep(0.0, max(comp.image_fade, 0.001), in.uv.y);
        let image_op =
            mix(comp.image_opacity_start, comp.image_opacity_end, img_t);
        wash_look = mix(base, wash_srgb, image_op);
    } else {
        // Soft highlight near the top-centre, trending into the dark base.
        let center = vec2<f32>(0.5, 0.22);
        var delta = in.uv - center;
        delta.x *= comp.target_size.x / comp.target_size.y;
        let glow = 1.0 - smoothstep(0.0, 0.75, length(delta));
        wash_look = mix(base, linear_to_srgb(comp.highlight.rgb), glow * 0.85);
    }

    // The base colour eases in over the wash from `bg_start`, with a hard floor
    // at `bg_solid`. The base fill stays opaque, so the window never is.
    let soft = smoothstep(comp.bg_start, max(comp.bg_end, comp.bg_start + 0.001), in.uv.y);
    let bg_op = mix(comp.bg_opacity_start, comp.bg_opacity_end, soft);
    // The floor snaps fully solid (independent of `bg_opacity_*`); a narrow
    // smoothstep softens the seam so its derivative doesn't read as a band.
    let floor = smoothstep(comp.bg_solid - 0.02, comp.bg_solid, in.uv.y);
    let coverage = max(bg_op, floor);
    var rgb = mix(wash_look, base, coverage);

    // Dither one 8-bit step in sRGB, then convert to linear so the surface's
    // hardware sRGB encode lands back on the dithered value.
    rgb = rgb + dither(in.position.xy) / 255.0;
    return vec4<f32>(srgb_to_linear(rgb), 1.0);
}
