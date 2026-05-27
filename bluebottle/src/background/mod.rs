mod shader;

use std::sync::Arc;

use bluebottle_ui::{color, style};
use iced::widget::shader::Shader;
use iced::{Color, Rectangle};

pub use self::shader::{CompositeKind, CompositeProgram, composite};
use crate::backdrop::BackdropImage;

/// Highlight colour for the gradient glow — a lifted, bluish tint of the base.
const HIGHLIGHT: Color = iced::color!(0x243154);

/// Marker selecting the main background's composite pipeline instance.
#[derive(Debug, Clone, Copy)]
pub struct BackgroundKind;

impl CompositeKind for BackgroundKind {
    const LABEL: &'static str = "background";
}

/// The live look parameters for the background.
///
/// The background sits on a solid (always opaque) primary fill. Over it: the
/// image is a faint translucent wash that eases out to the fill by `image_fade`,
/// and the fill colour eases further in over the wash between `bg_start` and
/// `bg_end`. The window is never transparent.
#[derive(Debug, Clone, Copy)]
pub struct BackgroundLook {
    /// Solid colour the page settles into; the sidebar leans this toward primary
    /// to read as a distinct shade.
    pub base: Color,
    /// Blur radius, in source pixels.
    pub blur: f32,
    /// Saturation multiplier for the wash (mirrors CSS `saturate()`).
    pub saturate: f32,
    /// Image wash opacity at the top, before `image_fade`.
    pub image_opacity_start: f32,
    /// Image wash opacity it eases to by `image_fade`.
    pub image_opacity_end: f32,
    /// Background-overlay opacity where it starts (at `bg_start`).
    pub bg_opacity_start: f32,
    /// Background-overlay opacity it reaches (at `bg_end` / `bg_solid`).
    pub bg_opacity_end: f32,
    /// Fraction down the screen by which the image wash has eased to its end
    /// opacity.
    pub image_fade: f32,
    /// Fraction down the screen at which the base colour begins easing further
    /// in over the wash.
    pub bg_start: f32,
    /// Fraction down the screen by which the soft base-colour fade reaches solid.
    pub bg_end: f32,
    /// Fraction down the screen at/below which the background is forced fully
    /// solid — a hard edge under the soft fade.
    pub bg_solid: f32,
    /// Vertical focal point for the image cover-fit (0 = top, 0.5 = centre),
    /// like CSS `background-position`'s vertical component.
    pub focus: f32,
    /// Zoom applied to the cover-fit image (1.0 = none), the overshoot that
    /// keeps the blur's soft edge off the bounds.
    pub zoom: f32,
}

impl Default for BackgroundLook {
    fn default() -> Self {
        Self {
            base: color::BACKGROUND,
            blur: style::IMAGE_BLUR,
            saturate: 1.4,
            image_opacity_start: 0.3,
            image_opacity_end: 0.0,
            bg_opacity_start: 0.0,
            bg_opacity_end: 1.0,
            image_fade: 0.5,
            bg_start: 0.0,
            bg_end: 0.6,
            bg_solid: 0.6,
            focus: 0.7,
            zoom: 1.15,
        }
    }
}

/// What the background derives its colours from.
#[derive(Debug)]
pub enum BackgroundSource {
    /// Blur and tint this backdrop image.
    Image(Arc<BackdropImage>),
    /// No image available: paint a procedural highlight-to-dark gradient.
    Gradient,
    /// No image available: fill solid with the look's base colour.
    Solid,
}

impl BackgroundSource {
    /// Picks the image source when one is available, else the gradient fallback.
    pub fn new(image: Option<BackdropImage>) -> Self {
        match image {
            Some(image) => Self::Image(Arc::new(image)),
            None => Self::Gradient,
        }
    }
}

/// A full-bleed background widget for the given `source` and `look`.
///
/// The `look` parameters are live; the blur is re-applied only when its radius
/// changes.
pub fn background<Message>(
    source: Arc<BackgroundSource>,
    look: BackgroundLook,
) -> Shader<Message, CompositeProgram<BackgroundKind>> {
    composite(source, look)
}

/// Packs the `Composite` uniform (see `background.wgsl`) for a `look` drawn over
/// `bounds`, where `mode` is `1.0` for a blurred image, `0.0` for the procedural
/// gradient, or `2.0` for a solid base fill, and `source_size` is the image size
/// in source pixels (or the bounds otherwise). Shared by the background and the
/// sidebar, which composite with the identical shader.
pub fn composite_uniform(
    look: BackgroundLook,
    mode: f32,
    source_size: [f32; 2],
    bounds: &Rectangle,
) -> [f32; 24] {
    let base = look.base.into_linear();
    let highlight = HIGHLIGHT.into_linear();
    [
        bounds.width,
        bounds.height,
        source_size[0],
        source_size[1],
        base[0],
        base[1],
        base[2],
        base[3],
        highlight[0],
        highlight[1],
        highlight[2],
        highlight[3],
        look.saturate,
        mode,
        look.image_opacity_start,
        look.image_opacity_end,
        look.bg_opacity_start,
        look.bg_opacity_end,
        look.image_fade,
        look.bg_start,
        look.bg_end,
        look.bg_solid,
        look.focus,
        look.zoom,
    ]
}
