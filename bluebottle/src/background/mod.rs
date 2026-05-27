//! The main screen background: a blurred backdrop image, or a procedural
//! gradient when no image is available.
//!
//! The look mirrors a two-layer CSS treatment — a heavily blurred, saturated
//! poster wash fading out down the page, under a dark vertical tint that lands
//! on the solid app background. It is drawn by a custom wgpu shader (see
//! [`shader`]); the [`background`] helper wraps it in an [`iced`] widget.

mod shader;

use std::sync::Arc;

use iced::widget::shader::Shader;
use iced::{Color, Length};

use self::shader::BackgroundPrimitive;
use crate::backdrop::BackdropImage;

/// Highlight colour for the gradient glow — a lifted, bluish tint of the base.
const HIGHLIGHT: Color = iced::color!(0x243154);

/// The live look parameters for the background.
///
/// The background sits on a solid (always opaque) primary fill. Over it: the
/// image is a faint translucent wash that eases out to the fill by `image_fade`,
/// and the fill colour eases further in over the wash between `bg_start` and
/// `bg_end`. The window is never transparent.
#[derive(Debug, Clone, Copy)]
pub struct Look {
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

impl Default for Look {
    fn default() -> Self {
        Self {
            blur: 45.0,
            saturate: 1.4,
            image_opacity_start: 0.1,
            image_opacity_end: 0.0,
            bg_opacity_start: 0.0,
            bg_opacity_end: 1.0,
            image_fade: 0.75,
            bg_start: 0.0,
            bg_end: 0.71,
            bg_solid: 0.5,
            focus: 0.5,
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
    look: Look,
) -> Shader<Message, BackgroundProgram> {
    Shader::new(BackgroundProgram { source, look })
        .width(Length::Fill)
        .height(Length::Fill)
}

/// The [`shader::Program`](iced::widget::shader::Program) driving the background.
pub struct BackgroundProgram {
    source: Arc<BackgroundSource>,
    look: Look,
}

impl<Message> iced::widget::shader::Program<Message> for BackgroundProgram {
    type State = ();
    type Primitive = BackgroundPrimitive;

    fn draw(
        &self,
        _state: &Self::State,
        _cursor: iced::mouse::Cursor,
        _bounds: iced::Rectangle,
    ) -> Self::Primitive {
        BackgroundPrimitive {
            source: Arc::clone(&self.source),
            look: self.look,
        }
    }
}
