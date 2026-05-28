// Shimmer skeleton. One shared pipeline draws every skeleton, and the highlight
// is a single diagonal band swept across the whole window in physical pixels, so
// every box lights up in unison as it passes. Each instance only varies its size
// and corner radius. This shader is self contained and does not share
// `shader_common.wgsl`.
//
// The resting colour arrives as sRGB. The band lifts it toward white by a fixed
// fraction, so the shimmer reads as a brightening on any surface colour rather
// than a mix toward one fixed tint. The result converts to linear for output,
// since the surface format encodes sRGB in hardware.

struct Shimmer {
    // Physical window size, the extent the band sweeps across.
    viewport: vec2<f32>,
    // Logical box size, for the rounded-corner coverage.
    box_size: vec2<f32>,
    // sRGB resting colour the box fills with.
    base_color: vec4<f32>,
    // Corner radius in logical pixels, clamped to half the short side.
    radius: f32,
    // Seconds since the shared clock anchor, already reduced modulo the cycle.
    time: f32,
    // Seconds for the band to cross the window once.
    cycle: f32,
    _pad0: f32,
}

@group(0) @binding(0) var<uniform> shimmer: Shimmer;

// Gaussian half-width of the band, in sweep-coordinate pixels.
const BAND: f32 = 240.0;
// How far the band lifts the resting colour toward white at its peak.
const LIFT: f32 = 0.13;

fn srgb_to_linear(c: vec3<f32>) -> vec3<f32> {
    let lo = c / 12.92;
    let hi = pow((c + 0.055) / 1.055, vec3<f32>(2.4));
    return select(hi, lo, c <= vec3<f32>(0.04045));
}

struct VsOut {
    @builtin(position) position: vec4<f32>,
    // Spans the box, with (0, 0) at the top-left.
    @location(0) uv: vec2<f32>,
}

// A full-screen triangle, the pass viewport clips it to the box.
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

@fragment
fn fs_shimmer(in: VsOut) -> @location(0) vec4<f32> {
    // The band sweeps a diagonal coordinate from off one edge to off the other,
    // so it is gone at both ends of the cycle and the loop is seamless.
    let span = shimmer.viewport.x + shimmer.viewport.y;
    let head = fract(shimmer.time / shimmer.cycle) * (span + 2.0 * BAND) - BAND;
    let coord = in.position.x + in.position.y;
    let offset = coord - head;
    let highlight = exp(-(offset * offset) / (2.0 * BAND * BAND));

    let base = shimmer.base_color.rgb;
    let peak = mix(base, vec3<f32>(1.0), LIFT);
    let color = mix(base, peak, highlight);

    // Rounded-box signed distance in the box's own pixels.
    let p = (in.uv - vec2<f32>(0.5)) * shimmer.box_size;
    let half_size = shimmer.box_size * 0.5;
    let r = min(shimmer.radius, min(half_size.x, half_size.y));
    let q = abs(p) - (half_size - vec2<f32>(r));
    let dist = length(max(q, vec2<f32>(0.0))) + min(max(q.x, q.y), 0.0) - r;

    // One-pixel antialiased edge.
    let alpha = 1.0 - smoothstep(0.0, fwidth(dist), dist);

    return vec4<f32>(srgb_to_linear(color), alpha);
}
