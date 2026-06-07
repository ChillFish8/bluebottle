// Composite stage for the `blurred_image` widget. Its own shader module
// so it redeclares the fullscreen triangle and its own bind layout.

struct VsOut {
    // Widget-local UV is derived in the fragment from `position.xy` and
    // the widget's physical bounds, so no extra varyings are forwarded.
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

// Substituted by the Rust pipeline so the array size and loop bound
// track `shader::MAX_REGIONS`.
const MAX_REGIONS: u32 = @MAX_REGIONS@u;

struct Composite {
    // Widget size in logical pixels. Regions are in this space.
    target_size: vec2<f32>,
    // Physical-pixel rect of the widget. Maps `gl_FragCoord` back into
    // widget-local UV so clipped viewports crop instead of squash.
    widget_origin_px: vec2<f32>,
    widget_size_px: vec2<f32>,
    region_count: u32,
    corner_radius: f32,
    // Progress strip painted along the bottom edge, masked by the outer
    // rounded alpha. Disabled when `progress_height` is 0.
    progress_fill: f32,
    progress_height: f32,
    progress_color: vec4<f32>,
    progress_track: vec4<f32>,
    // Each region is (x, y, width, height) in widget-local pixels.
    regions: array<vec4<f32>, @MAX_REGIONS@>,
    // Per-region corner radius, four packed per vec4. Pass a very large
    // value to render the region as a pill. Clamped inside the SDF.
    region_radii: array<vec4<f32>, @MAX_REGIONS_DIV_4@>,
}

@group(0) @binding(0) var<uniform> comp: Composite;
@group(0) @binding(1) var sharp_tex: texture_2d<f32>;
@group(0) @binding(2) var blurred_tex: texture_2d<f32>;
@group(0) @binding(3) var comp_sampler: sampler;

// Signed distance from `p` to a rounded rectangle. Negative inside, zero
// on the edge, positive outside.
fn rounded_rect_sdf(
    p: vec2<f32>,
    center: vec2<f32>,
    half: vec2<f32>,
    radius: f32,
) -> f32 {
    let q = abs(p - center) - max(half - vec2<f32>(radius, radius), vec2<f32>(0.0));
    return length(max(q, vec2<f32>(0.0))) + min(max(q.x, q.y), 0.0) - radius;
}

// Region coverage at pixel `p`. Radius is clamped so a large value gives
// a pill instead of a negative inset.
fn region_coverage(p: vec2<f32>, rect: vec4<f32>, radius: f32) -> f32 {
    let center = rect.xy + rect.zw * 0.5;
    let half = rect.zw * 0.5;
    let max_radius = min(half.x, half.y);
    let r = min(radius, max_radius);
    let sdf = rounded_rect_sdf(p, center, half, r);
    // Soft 1 px edge for antialiasing.
    return 1.0 - clamp(sdf + 0.5, 0.0, 1.0);
}

// Look up the i-th region's per-region radius from the packed vec4 array.
fn region_radius(i: u32) -> f32 {
    let v = comp.region_radii[i / 4u];
    let lane = i % 4u;
    if lane == 0u { return v.x; }
    if lane == 1u { return v.y; }
    if lane == 2u { return v.z; }
    return v.w;
}

@fragment
fn fs_composite(in: VsOut) -> @location(0) vec4<f32> {
    // `in.position.xy` is the physical fragment position. The clamp
    // guards a transient zero-sized layout against NaN.
    let safe_size = max(comp.widget_size_px, vec2<f32>(1.0, 1.0));
    let widget_uv = (in.position.xy - comp.widget_origin_px) / safe_size;
    let pixel = widget_uv * comp.target_size;

    var coverage = 0.0;
    let n = min(comp.region_count, MAX_REGIONS);
    for (var i = 0u; i < n; i = i + 1u) {
        let c = region_coverage(pixel, comp.regions[i], region_radius(i));
        coverage = max(coverage, c);
    }

    let sharp = textureSampleLevel(sharp_tex, comp_sampler, widget_uv, 0.0);
    let blurred = textureSampleLevel(blurred_tex, comp_sampler, widget_uv, 0.0);
    var rgb = mix(sharp.rgb, blurred.rgb, coverage);

    // Darken the frosted regions for chrome contrast. Coverage-scaled so
    // the darken feathers in with the rounded SDF.
    rgb = mix(rgb, vec3<f32>(0.0, 0.0, 0.0), coverage * 0.3);

    // Progress strip. Painted before the outer mask so it inherits the
    // image's rounded clip on the bottom corners.
    if comp.progress_height > 0.0 {
        let strip_top = comp.target_size.y - comp.progress_height;
        if pixel.y >= strip_top {
            let fill_x = comp.target_size.x * comp.progress_fill;

            // Top-right corner rounds in by half the strip height while
            // the bar is below 100%. At 100% the radius eases to zero so
            // the edge meets the image's outer mask cleanly. Capped at
            // the current fill width so early slivers still round.
            let cap_r = comp.progress_height * 0.5
                * clamp(1.0 - comp.progress_fill, 0.0, 1.0);
            let corner_r = min(cap_r, fill_x);
            let corner_center =
                vec2<f32>(fill_x - corner_r, strip_top + corner_r);

            var filled_cov = 0.0;
            if pixel.x < fill_x {
                filled_cov = 1.0;
                if corner_r > 0.0
                    && pixel.x > corner_center.x
                    && pixel.y < corner_center.y
                {
                    let d = distance(pixel, corner_center);
                    filled_cov = 1.0 - clamp(d - corner_r + 0.5, 0.0, 1.0);
                }
            }

            // Track first, then the accent cap on top so the track fills
            // the notch above the curved edge.
            rgb = mix(rgb, comp.progress_track.rgb, comp.progress_track.a);
            if filled_cov > 0.0 {
                rgb = mix(
                    rgb,
                    comp.progress_color.rgb,
                    comp.progress_color.a * filled_cov,
                );
            }

            // 1 px sheen along the top edge of the filled portion. The
            // brightened-accent stays in the accent's hue. Coverage gate
            // keeps the sheen following the rounded corner.
            if pixel.y - strip_top < 1.0 && filled_cov > 0.0 {
                let highlight = min(
                    comp.progress_color.rgb * 1.45,
                    vec3<f32>(1.0, 1.0, 1.0),
                );
                rgb = mix(rgb, highlight, filled_cov);
            }
        }
    }

    // Outer rounded mask so the image clips itself instead of bleeding
    // past whatever rounded chrome the caller paints over it.
    let outer_rect = vec4<f32>(0.0, 0.0, comp.target_size.x, comp.target_size.y);
    let outer_alpha = region_coverage(pixel, outer_rect, comp.corner_radius);
    return vec4<f32>(rgb, outer_alpha);
}
