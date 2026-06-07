// Composite stage for the `blurred_image` widget. This is its own shader
// module, so it redeclares the fullscreen triangle and its own bind layout
// rather than sharing one with the blur pipeline.

struct VsOut {
    // The fragment shader derives widget-local UV from `position.xy`
    // (framebuffer pixels) and the widget's physical bounds in the uniform,
    // so no extra varyings are forwarded from here.
    @builtin(position) position: vec4<f32>,
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
    return out;
}

// The cap is substituted by the Rust pipeline at compile time so the array
// size and loop bound track `shader::MAX_REGIONS` automatically.
const MAX_REGIONS: u32 = @MAX_REGIONS@u;

struct Composite {
    // Widget size, in logical pixels. Regions are expressed in this space.
    target_size: vec2<f32>,
    // The widget's top-left and size in physical surface pixels. The shader
    // uses these to map a fragment's `gl_FragCoord` back into widget-local
    // UV space, so partial visibility (scrollables, narrow windows, anything
    // that clips the viewport) crops the image at its natural anchoring
    // instead of squashing the whole widget into the visible region.
    widget_origin_px: vec2<f32>,
    widget_size_px: vec2<f32>,
    region_count: u32,
    corner_radius: f32,
    // Each region is (x, y, width, height) in widget-local pixels.
    regions: array<vec4<f32>, @MAX_REGIONS@>,
}

@group(0) @binding(0) var<uniform> comp: Composite;
@group(0) @binding(1) var sharp_tex: texture_2d<f32>;
@group(0) @binding(2) var blurred_tex: texture_2d<f32>;
@group(0) @binding(3) var comp_sampler: sampler;

// Signed distance from `p` to a rectangle centred at `center` with half-extents
// `half`, rounded by `radius`. Negative inside, zero on the edge, positive
// outside.
fn rounded_rect_sdf(
    p: vec2<f32>,
    center: vec2<f32>,
    half: vec2<f32>,
    radius: f32,
) -> f32 {
    let q = abs(p - center) - max(half - vec2<f32>(radius, radius), vec2<f32>(0.0));
    return length(max(q, vec2<f32>(0.0))) + min(max(q.x, q.y), 0.0) - radius;
}

// Coverage of one region at pixel `p`, clamped so an over-large corner radius
// degrades to the rect's smallest half-extent rather than producing a negative
// rounded inset.
fn region_coverage(p: vec2<f32>, rect: vec4<f32>, radius: f32) -> f32 {
    let center = rect.xy + rect.zw * 0.5;
    let half = rect.zw * 0.5;
    let max_radius = min(half.x, half.y);
    let r = min(radius, max_radius);
    let sdf = rounded_rect_sdf(p, center, half, r);
    // Soft 1px edge so the rounded corner antialiases against the sharp image.
    return 1.0 - clamp(sdf + 0.5, 0.0, 1.0);
}

@fragment
fn fs_composite(in: VsOut) -> @location(0) vec4<f32> {
    // `in.position.xy` is the physical fragment position on the surface, so
    // it stays anchored to the widget even when the viewport only covers a
    // clipped slice of it. The `max(..., 1.0)` guard keeps a transient
    // zero-sized layout from producing NaN here and corrupting the SDF below.
    let safe_size = max(comp.widget_size_px, vec2<f32>(1.0, 1.0));
    let widget_uv = (in.position.xy - comp.widget_origin_px) / safe_size;
    let pixel = widget_uv * comp.target_size;
    var coverage = 0.0;
    let n = min(comp.region_count, MAX_REGIONS);
    for (var i = 0u; i < n; i = i + 1u) {
        let c = region_coverage(pixel, comp.regions[i], comp.corner_radius);
        coverage = max(coverage, c);
    }
    let sharp = textureSampleLevel(sharp_tex, comp_sampler, widget_uv, 0.0);
    let blurred = textureSampleLevel(blurred_tex, comp_sampler, widget_uv, 0.0);
    let rgb = mix(sharp.rgb, blurred.rgb, coverage);
    return vec4<f32>(rgb, 1.0);
}
