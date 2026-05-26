//! The main screen background: a blurred spotlight image, or a procedural
//! gradient when no image is cached.
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
use crate::spotlight::SpotlightImage;

/// Highlight colour for the gradient glow — a lifted, bluish tint of the base.
const HIGHLIGHT: Color = iced::color!(0x243154);

/// Default blur radius, in source pixels (mirrors the CSS `blur(40px)`).
pub const DEFAULT_BLUR: f32 = 40.0;
/// Default saturation boost for the poster wash (mirrors `saturate(1.4)`).
pub const DEFAULT_SATURATE: f32 = 1.4;

/// What the background derives its colours from.
#[derive(Debug)]
pub enum BackgroundSource {
    /// Blur and tint this cached spotlight image.
    Image(Arc<SpotlightImage>),
    /// No image cached: paint a procedural highlight-to-dark gradient.
    Gradient,
}

impl BackgroundSource {
    /// Picks the image source when one is cached, else the gradient fallback.
    pub fn new(image: Option<SpotlightImage>) -> Self {
        match image {
            Some(image) => Self::Image(Arc::new(image)),
            None => Self::Gradient,
        }
    }
}

/// A full-bleed background widget for the given `source`.
///
/// `blur` and `saturate` are live parameters; the blur is re-applied only when
/// its radius changes.
pub fn background<Message>(
    source: Arc<BackgroundSource>,
    blur: f32,
    saturate: f32,
) -> Shader<Message, BackgroundProgram> {
    Shader::new(BackgroundProgram {
        source,
        blur,
        saturate,
    })
    .width(Length::Fill)
    .height(Length::Fill)
}

/// The [`shader::Program`](iced::widget::shader::Program) driving the background.
pub struct BackgroundProgram {
    source: Arc<BackgroundSource>,
    blur: f32,
    saturate: f32,
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
            blur: self.blur,
            saturate: self.saturate,
        }
    }
}
