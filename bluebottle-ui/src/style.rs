use std::time::Duration;

use iced::{Border, Color, Shadow, Vector};

use crate::color;
use crate::util::lerp;

/// Blur radius for frosted image backdrops (the page background and overlay
/// panels), in source pixels.
pub const IMAGE_BLUR: f32 = 35.0;

/// How long the background takes to fade in or crossfade between images.
pub const CROSSFADE: Duration = Duration::from_millis(300);

/// Bordered-glass emphasis animation. The switch knob slide, the animated
/// tick stroke, and the clickable's selected fade for the switch chassis
/// all share this so settings rows that pair a checkbox and a switch read
/// as one family.
pub const EMPHASIS: Duration = Duration::from_millis(220);

/// Peak coverage of the image-less background glow over its base color.
pub const GLOW_STRENGTH: f32 = 0.6;

/// Width of an overlay panel's leading-edge accent line, in logical pixels.
pub const BORDER_WIDTH: f32 = 1.0;

// Shadow policy. Three recipes drive every elevated surface.
//
// 1. Neutral elevation. Soft black drop. The CSS source uses a negative
//    `spread` to keep the blur tucked under the element. iced's `Shadow` has
//    no spread, so each tier records `blur - |spread|` as the blur radius
//    instead. The shadow reads a touch larger than the CSS equivalent but
//    stays in the same family.
//
// 2. Accent glow. The active accent at a fixed alpha. Built at frame time
//    via [`glow`] so it tracks accent swaps.
//
// 3. Hairline ring. A 1px coloured outline. iced has no inset/spread shadow,
//    so rings are applied as a `Border` via [`hairline`], not a `Shadow`.

/// Sidebar drawer's tucked edge shadow.
/// CSS `-14px 0 38px -18px rgba(0,0,0,.4)`.
pub const SIDEBAR_DROP: Shadow = Shadow {
    color: color::with_alpha(Color::BLACK, 0.4),
    offset: Vector { x: -14.0, y: 0.0 },
    blur_radius: 20.0,
};

/// Resting card / popover elevation.
/// CSS `0 12px 24px -8px rgba(0,0,0,.5)`.
pub const ELEVATION_RESTING: Shadow = Shadow {
    color: color::with_alpha(Color::BLACK, 0.5),
    offset: Vector { x: 0.0, y: 12.0 },
    blur_radius: 16.0,
};

/// Hovered / lifted card elevation.
/// CSS `0 30px 60px -10px rgba(0,0,0,.7)`.
pub const ELEVATION_LIFTED: Shadow = Shadow {
    color: color::with_alpha(Color::BLACK, 0.7),
    offset: Vector { x: 0.0, y: 30.0 },
    blur_radius: 50.0,
};

/// Accent glow at `alpha`. Use 0.40 for primary buttons, 0.53 for hovered
/// posters, 0.67 for play FABs. The opacity is authored in sRGB and converted
/// to the renderer's linear alpha. Reads the active accent at call time.
pub fn glow(alpha: f32) -> Shadow {
    Shadow {
        color: color::with_alpha(color::primary(), color::srgb_alpha(alpha)),
        offset: Vector { x: 0.0, y: 10.0 },
        blur_radius: 24.0,
    }
}

/// Hero button glow. `fill` is the button's own fill colour so the glow reads
/// as the button bleeding light. It eases by `factor` from its resting spread
/// to the larger, brighter hover spread. The opacities are authored in sRGB.
/// Rest is `0 8px 24px` at 8%, hover is `0 10px 28px` at 14%.
pub fn hero_glow(fill: Color, factor: f32) -> Shadow {
    Shadow {
        color: color::with_alpha(fill, color::srgb_alpha(lerp(0.08, 0.14, factor))),
        offset: Vector::new(0.0, lerp(8.0, 10.0, factor)),
        blur_radius: lerp(24.0, 28.0, factor),
    }
}

/// 1px hairline ring. Pair with a [`Shadow`] for the "drop + ring" recipe.
/// Pass [`color::border`] or [`color::border_strong`] for neutral rings, or
/// [`color::primary()`] for selection / hover accents.
pub fn hairline(color: Color) -> Border {
    Border::default().width(1.0).color(color)
}

/// Returns `shadow` with its colour alpha, offset, and blur radius all scaled
/// by `factor`. Use this to animate a standard [`Shadow`] from no elevation
/// at `factor = 0.0` to full elevation at `factor = 1.0`.
pub fn scale_shadow(shadow: Shadow, factor: f32) -> Shadow {
    Shadow {
        color: color::with_alpha(shadow.color, lerp(0.0, shadow.color.a, factor)),
        offset: Vector::new(
            lerp(0.0, shadow.offset.x, factor),
            lerp(0.0, shadow.offset.y, factor),
        ),
        blur_radius: lerp(0.0, shadow.blur_radius, factor),
    }
}
