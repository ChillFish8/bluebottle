// Shimmer skeleton. One shared pipeline draws every skeleton, and the highlight
// is a single diagonal band swept across the whole window in physical pixels, so
// every box lights up in unison as it passes. Each instance only varies its size
// and corner radius. This shader is self contained and does not share
// `shader_common.wgsl`.
//
// The fill is the design system's bordered glass identity. A faint white-glass
// fill sits behind a hairline ring just inside the rounded edge, so the
// skeleton brightens whatever surface it lands on rather than stamping a fixed
// colour on top. The shimmer lifts the fill's alpha at the band peak, so the
// sweep reads as the same glass surface brightening rather than a tint swap.
// Both fill and ring author their opacities in sRGB on the Rust side, the
// blend happens in linear, and the output converts to linear here since the
// surface format encodes sRGB in hardware.

struct Shimmer {
    // Physical window size, the extent the band sweeps across.
    viewport: vec2<f32>,
    // Logical box size, for the rounded-corner coverage.
    box_size: vec2<f32>,
    // sRGB resting fill. White rgb with a linear-space alpha.
    base_color: vec4<f32>,
    // sRGB hairline ring. White rgb with a linear-space alpha.
    border_color: vec4<f32>,
    // Corner radius in logical pixels, clamped to half the short side.
    radius: f32,
    // Seconds since the shared clock anchor, already reduced modulo the cycle.
    time: f32,
    // Seconds for the band to cross the window once.
    cycle: f32,
    // Additional fill alpha at the shimmer peak. Resting alpha plus this is the
    // peak alpha. Both are in the same linear-space units as `base_color.a`.
    peak_lift: f32,
}

@group(0) @binding(0) var<uniform> shimmer: Shimmer;

// Gaussian half-width of the band, in sweep-coordinate pixels.
const BAND: f32 = 240.0;
// Hairline thickness in logical pixels.
const BORDER_WIDTH: f32 = 1.0;

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

    // Rounded-box signed distance in the box's own pixels.
    let p = (in.uv - vec2<f32>(0.5)) * shimmer.box_size;
    let half_size = shimmer.box_size * 0.5;
    let r = min(shimmer.radius, min(half_size.x, half_size.y));
    let q = abs(p) - (half_size - vec2<f32>(r));
    let dist = length(max(q, vec2<f32>(0.0))) + min(max(q.x, q.y), 0.0) - r;

    // Outer edge antialiases over one pixel. The interior excludes a one-pixel
    // ring just inside that edge, and ring is the difference, so the two masks
    // together exactly cover the rounded box.
    let aa = fwidth(dist);
    let outer = 1.0 - smoothstep(0.0, aa, dist);
    let inner = 1.0 - smoothstep(0.0, aa, dist + BORDER_WIDTH);
    let ring = max(outer - inner, 0.0);

    // The fill alpha lifts at the band peak, the ring alpha stays constant so
    // the hairline does not flare.
    let fill_alpha = shimmer.base_color.a + shimmer.peak_lift * highlight;

    // Fill and ring share white rgb, so their separate contributions sum into
    // one straight-alpha output without proper alpha-over compositing.
    let combined_alpha = fill_alpha * inner + shimmer.border_color.a * ring;

    return vec4<f32>(srgb_to_linear(shimmer.base_color.rgb), combined_alpha);
}
