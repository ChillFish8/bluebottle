// Inspect-modal scrim composite stage. Concatenated after `shader_common.wgsl`,
// which provides `VsOut`, `vs_fullscreen`, the `Blur` pre-pass (`fs_blur`), and
// the sRGB / dither helpers.
//
// This does both modal layers in one pass over the blurred snapshot: a dark tint
// everywhere, and a near-solid, primary-hued pane inside a centered rounded rect
// (the panel) sampling a heavier-blurred copy. Colour maths matches the
// background: blur in linear, tint/dither in sRGB, output linear for the
// surface's sRGB encode.

struct Scrim {
    // Tint outside the panel: sRGB rgb in `.xyz`, coverage in `.w`.
    scrim_tint: vec4<f32>,
    // Tint inside the rounded panel rect: sRGB rgb in `.xyz`, coverage in `.w`.
    panel_tint: vec4<f32>,
    // Physical pixel size of the whole scrim (the window).
    target_px: vec2<f32>,
    // Physical pixel size of the centered panel rect.
    panel_px: vec2<f32>,
    // Overall output alpha — the fade factor in [0, 1].
    factor: f32,
    // Panel corner radius, in physical pixels.
    corner_px: f32,
    // Saturation multiplier for the blurred scene (1 = unchanged).
    saturate: f32,
    _pad0: f32,
    // Snapshot size in physical pixels, to cover-fit it to a resized window.
    source_px: vec2<f32>,
    _pad1: f32,
    _pad2: f32,
}

@group(0) @binding(0) var<uniform> scrim: Scrim;
// The scene blurred at the scrim radius; `panel_scene` is the same blurred
// further (compounded) for a visibly heavier blur inside the panel.
@group(0) @binding(1) var scene: texture_2d<f32>;
@group(0) @binding(2) var scene_sampler: sampler;
@group(0) @binding(3) var panel_scene: texture_2d<f32>;

// Signed distance from `p` to a rounded rectangle centred at the origin.
fn rounded_box_sdf(p: vec2<f32>, half: vec2<f32>, r: f32) -> f32 {
    let q = abs(p) - half + vec2<f32>(r);
    return length(max(q, vec2<f32>(0.0))) + min(max(q.x, q.y), 0.0) - r;
}

// Saturation around luma (linear space), mirroring the background's wash.
fn saturate_color(c: vec3<f32>, amount: f32) -> vec3<f32> {
    let luma = dot(c, vec3<f32>(0.2126, 0.7152, 0.0722));
    return mix(vec3<f32>(luma), c, amount);
}

// Cover-fit the snapshot to the window (centred), so a resized window crops and
// zooms rather than stretching the fixed-size capture. Identity when aspects match.
fn cover_uv(uv: vec2<f32>) -> vec2<f32> {
    let target_aspect = scrim.target_px.x / scrim.target_px.y;
    let source_aspect = scrim.source_px.x / scrim.source_px.y;
    var centered = uv - vec2<f32>(0.5);
    if target_aspect > source_aspect {
        centered.y *= source_aspect / target_aspect;
    } else {
        centered.x *= target_aspect / source_aspect;
    }
    return centered + vec2<f32>(0.5);
}

@fragment
fn fs_scrim(in: VsOut) -> @location(0) vec4<f32> {
    // Both blurred snapshots, cover-fit to the window: the scrim-blur for outside
    // the panel, the heavier compounded blur for inside it.
    let scene_uv = cover_uv(in.uv);
    let scrim_lin =
        saturate_color(textureSampleLevel(scene, scene_sampler, scene_uv, 0.0).rgb, scrim.saturate);
    let panel_lin =
        saturate_color(textureSampleLevel(panel_scene, scene_sampler, scene_uv, 0.0).rgb, scrim.saturate);
    let scrim_srgb = linear_to_srgb(scrim_lin);
    let panel_srgb = linear_to_srgb(panel_lin);

    // Coverage of the centred rounded panel rect (1 inside, 0 out, ~1px AA).
    let p = (in.uv - vec2<f32>(0.5)) * scrim.target_px;
    let d = rounded_box_sdf(p, scrim.panel_px * 0.5, scrim.corner_px);
    let in_panel = clamp(0.5 - d, 0.0, 1.0);

    // Outside the panel: tint the scrim blur in sRGB, like the background
    // composite and CSS's `rgba()` compositing.
    let outside = mix(scrim_srgb, scrim.scrim_tint.xyz, scrim.scrim_tint.w);
    // Inside: a near-solid panel tint (basically the app background) over the
    // heavier blur, which shows through only faintly. `panel_tint.w` is the
    // tint's opacity over the blur.
    let inside = mix(panel_srgb, scrim.panel_tint.xyz, scrim.panel_tint.w);
    var rgb = mix(outside, inside, in_panel);
    rgb = rgb + dither(in.position.xy) / 255.0;

    // Straight-alpha so `factor` blends the scrim in over the still-crisp scene
    // beneath it; back to linear for the sRGB surface encode.
    return vec4<f32>(srgb_to_linear(rgb), scrim.factor);
}
