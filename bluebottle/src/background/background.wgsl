// Background composite stage. Concatenated after `shader_common.wgsl`, which
// provides `VsOut`, `vs_fullscreen`, the `Blur` pre-pass (`fs_blur`), and the
// sRGB / dither helpers. This lays the blurred poster (or a procedural gradient)
// under a dark vertical tint.
//
// Colour maths runs in linear space: source and intermediate targets are sRGB
// textures, and the constant colours arrive as linear uniforms.

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

@fragment
fn fs_composite(in: VsOut) -> @location(0) vec4<f32> {
    // Composite in sRGB (display) space to match CSS gradient interpolation:
    // blur and saturate run in linear, then everything converts to sRGB for the
    // opacity/coverage mixes and back to linear for output.
    let base = linear_to_srgb(comp.base_color.rgb);

    // The wash at the top: a solid base, the image knocked back by its opacity
    // so the base shows through, or the procedural highlight, over the base fill.
    var wash_look: vec3<f32>;
    if comp.mode > 1.5 {
        // Solid fill: the base everywhere (the coverage mix below is a no-op).
        wash_look = base;
    } else if comp.mode > 0.5 {
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
