use std::time::Duration;

use iced::{Shadow, Vector};

use crate::color;

/// Blur radius for frosted image backdrops (the page background and overlay
/// panels), in source pixels.
pub const IMAGE_BLUR: f32 = 35.0;

/// How long the background takes to fade in or crossfade between images.
pub const CROSSFADE: Duration = Duration::from_millis(300);

/// Peak coverage of the image-less background glow over its base color.
pub const GLOW_STRENGTH: f32 = 0.6;

/// Width of an overlay panel's leading-edge accent line, in logical pixels.
pub const BORDER_WIDTH: f32 = 1.5;

/// Elevation shadow an overlay panel casts off its leading (left) edge.
pub const PANEL_SHADOW: Shadow = Shadow {
    color: color::with_alpha(color::BG, 0.5),
    offset: Vector { x: -8.0, y: 0.0 },
    blur_radius: 24.0,
};

/// A subtle drop shadow used to lift small surfaces (cards, popovers,
/// tooltips) when they enter a hover or focus state. Animated effects scale
/// it with [`scale_shadow`].
pub const ELEVATION_SHADOW: Shadow = Shadow {
    color: color::with_alpha(iced::Color::BLACK, 0.35),
    offset: Vector { x: 0.0, y: 2.0 },
    blur_radius: 6.0,
};

/// Returns `shadow` with its colour alpha, offset, and blur radius all scaled
/// by `factor`. Use this to animate a standard [`Shadow`] from no elevation
/// at `factor = 0.0` to full elevation at `factor = 1.0`.
pub fn scale_shadow(shadow: Shadow, factor: f32) -> Shadow {
    Shadow {
        color: color::with_alpha(shadow.color, shadow.color.a * factor),
        offset: Vector::new(shadow.offset.x * factor, shadow.offset.y * factor),
        blur_radius: shadow.blur_radius * factor,
    }
}
