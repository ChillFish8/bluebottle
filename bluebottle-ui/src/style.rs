use iced::{Shadow, Vector};

use crate::color;

/// Blur radius for frosted image backdrops (the page background and overlay
/// panels), in source pixels.
pub const IMAGE_BLUR: f32 = 45.0;

/// Width of an overlay panel's leading-edge accent line, in logical pixels.
pub const BORDER_WIDTH: f32 = 1.5;

/// Elevation shadow an overlay panel casts off its leading (left) edge.
pub const PANEL_SHADOW: Shadow = Shadow {
    color: color::with_alpha(color::BACKGROUND, 0.5),
    offset: Vector { x: -8.0, y: 0.0 },
    blur_radius: 24.0,
};
