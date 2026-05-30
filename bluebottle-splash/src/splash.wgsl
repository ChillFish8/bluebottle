// Full screen splash. A centred logo over a solid background with a small white
// ring spinner a fixed gap below it. Everything is drawn in one fragment pass off
// a three vertex full screen triangle. Pixel space has its origin at the top left.

struct Uniforms {
    // Surface size in physical pixels.
    resolution: vec2<f32>,
    // Seconds since the renderer started, driving the spinner rotation.
    time: f32,
    // Overall opacity, 1.0 fully shown and 0.0 fully faded out.
    fade: f32,
    // Logo placement in pixels as x, y, width, height.
    logo_rect: vec4<f32>,
    // Spinner as centre x, centre y, radius, ring thickness.
    spinner: vec4<f32>,
    // Background fill in linear rgba.
    background: vec4<f32>,
};

@group(0) @binding(0) var<uniform> u: Uniforms;
@group(0) @binding(1) var logo_tex: texture_2d<f32>;
@group(0) @binding(2) var logo_samp: sampler;

const TWO_PI: f32 = 6.2831853;
// Fraction of a turn the visible arc covers and how fast it sweeps.
const ARC_LEN: f32 = 4.6;
const SPEED: f32 = 4.0;

@vertex
fn vs(@builtin(vertex_index) index: u32) -> @builtin(position) vec4<f32> {
    let uv = vec2<f32>(f32((index << 1u) & 2u), f32(index & 2u));
    return vec4<f32>(uv * 2.0 - 1.0, 0.0, 1.0);
}

@fragment
fn fs(@builtin(position) frag: vec4<f32>) -> @location(0) vec4<f32> {
    let px = frag.xy;
    var color = u.background.rgb;

    // Logo, alpha blended over the background within its rectangle.
    let lo = u.logo_rect;
    if lo.z > 0.0 {
        let uv = (px - lo.xy) / lo.zw;
        if uv.x >= 0.0 && uv.x <= 1.0 && uv.y >= 0.0 && uv.y <= 1.0 {
            let tex = textureSampleLevel(logo_tex, logo_samp, uv, 0.0);
            color = mix(color, tex.rgb, tex.a);
        }
    }

    // Ring spinner, a rotating arc with a soft leading edge.
    let centre = u.spinner.xy;
    let radius = u.spinner.z;
    let thickness = u.spinner.w;
    let d = distance(px, centre);
    let ring = 1.0 - smoothstep(thickness, thickness + 1.5, abs(d - radius));

    let angle = atan2(px.y - centre.y, px.x - centre.x);
    var swept = angle - u.time * SPEED;
    swept = swept - floor(swept / TWO_PI) * TWO_PI;
    let head = smoothstep(0.0, 0.35, swept);
    let tail = 1.0 - smoothstep(ARC_LEN, ARC_LEN + 0.35, swept);
    let arc = head * tail;

    let spinner = clamp(ring * arc, 0.0, 1.0);
    color = mix(color, vec3<f32>(1.0, 1.0, 1.0), spinner);

    // The splash is opaque, so its premultiplied alpha is the fade itself. The
    // compositor blends this over the UI beneath, giving a fade out on load.
    return vec4<f32>(color * u.fade, u.fade);
}
