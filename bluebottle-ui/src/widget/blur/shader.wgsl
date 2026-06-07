// Shared scaffolding for shader widgets that lean on a separable Gaussian
// blur. The fullscreen triangle, the blur pre-pass, and the sRGB / dither
// helpers all live here. Consumers that share one shader module with their
// own composite stage concatenate this file ahead of theirs and redeclare
// their own uniform and textures at the same binding slots.

struct VsOut {
    @builtin(position) position: vec4<f32>,
    // Spans the widget rect, with (0, 0) at the top-left.
    @location(0) uv: vec2<f32>,
}

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
    // source texel. Coarser spacing reads as stepping once up-scaled.
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

// sRGB transfer functions, used to composite/dither in display space (where the
// 8-bit quantisation happens) rather than in linear space.
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
