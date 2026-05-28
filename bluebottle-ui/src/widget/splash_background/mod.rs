//! A full-bleed frosted background widget.
//!
//! It blurs and tints a backdrop image, or paints a soft glow when there is no
//! image. The widget drives its own fade-in and crossfade animations, so the
//! application does not need to tick it.

mod gpu;
mod shader;

use std::fmt;
use std::marker::PhantomData;
use std::sync::Arc;

use iced::widget::shader::Shader;
use iced::{Color, Element, Length};

use crate::{color, style};

/// The background over the page, settling into the app background color.
pub fn splash_background(image: Option<Backdrop>) -> SplashBackground<Background> {
    SplashBackground {
        image,
        look: Look::default(),
        kind: PhantomData,
    }
}

/// The page-background look re-tinted onto a deep gradient. The image wash
/// eases out a touch earlier so the panel reads as a distinct shade.
pub fn splash_panel(image: Option<Backdrop>) -> SplashBackground<Panel> {
    SplashBackground {
        image,
        look: Look {
            base_top: Color::from_rgba8(28, 22, 60, 0.92),
            base_bottom: Color::from_rgba8(20, 18, 42, 0.96),
            image_fade: 0.4,
            ..Look::default()
        },
        kind: PhantomData,
    }
}

/// A decoded backdrop image, kept as packed RGBA8 for GPU upload.
///
/// Cloning shares the pixels, so a clone stays the same image to the widget and
/// does not trigger a crossfade.
#[derive(Clone)]
pub struct Backdrop {
    inner: Arc<Pixels>,
}

struct Pixels {
    rgba: Vec<u8>,
    width: u32,
    height: u32,
}

impl Backdrop {
    /// Wraps `width` x `height` row-major RGBA8 pixels.
    pub fn new(rgba: Vec<u8>, width: u32, height: u32) -> Self {
        Self {
            inner: Arc::new(Pixels {
                rgba,
                width,
                height,
            }),
        }
    }

    fn rgba(&self) -> &[u8] {
        &self.inner.rgba
    }

    fn width(&self) -> u32 {
        self.inner.width
    }

    fn height(&self) -> u32 {
        self.inner.height
    }

    /// Identity of the shared pixels, used to detect when the image changes.
    fn key(&self) -> usize {
        Arc::as_ptr(&self.inner) as usize
    }
}

impl fmt::Debug for Backdrop {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Backdrop")
            .field("width", &self.inner.width)
            .field("height", &self.inner.height)
            .field("bytes", &self.inner.rgba.len())
            .finish()
    }
}

/// The look parameters for a background.
///
/// The image is a faint wash that eases out to the base color by `image_fade`,
/// and the base color eases in over the wash between `bg_start` and `bg_end`,
/// snapping fully solid at `bg_solid`. The base is a vertical gradient from
/// `base_top` to `base_bottom`. Set both to the same colour for a flat base.
#[derive(Debug, Clone, Copy)]
pub struct Look {
    /// Base colour at the top of the surface.
    pub base_top: Color,
    /// Base colour at the bottom of the surface.
    pub base_bottom: Color,
    /// Blur radius, in source pixels.
    pub blur: f32,
    /// Saturation multiplier for the image wash.
    pub saturate: f32,
    /// Image wash opacity at the top, before `image_fade`.
    pub image_opacity_start: f32,
    /// Image wash opacity it eases to by `image_fade`.
    pub image_opacity_end: f32,
    /// Base-overlay opacity where it starts, at `bg_start`.
    pub bg_opacity_start: f32,
    /// Base-overlay opacity it reaches, at `bg_end`.
    pub bg_opacity_end: f32,
    /// Fraction down the surface by which the image wash has eased out.
    pub image_fade: f32,
    /// Fraction down the surface at which the base color begins easing in.
    pub bg_start: f32,
    /// Fraction down the surface by which the soft base fade reaches solid.
    pub bg_end: f32,
    /// Fraction at/below which the base is forced fully solid, a hard edge.
    pub bg_solid: f32,
    /// Vertical focal point for the cover-fit, 0 = top, 0.5 = centre.
    pub focus: f32,
    /// Zoom applied to the cover-fit image, 1.0 = none.
    pub zoom: f32,
    /// Glow color painted when there is no image.
    pub glow: Color,
    /// Peak coverage of the glow over the base.
    pub glow_strength: f32,
}

impl Default for Look {
    fn default() -> Self {
        Self {
            base_top: color::BACKGROUND,
            base_bottom: color::BACKGROUND,
            blur: style::IMAGE_BLUR,
            saturate: 1.4,
            image_opacity_start: 0.5,
            image_opacity_end: 0.0,
            bg_opacity_start: 0.0,
            bg_opacity_end: 1.0,
            image_fade: 0.5,
            bg_start: 0.45,
            bg_end: 0.85,
            bg_solid: 0.85,
            focus: 0.3,
            zoom: 1.15,
            glow: color::GLOW,
            glow_strength: style::GLOW_STRENGTH,
        }
    }
}

/// Selects which pipeline instance a background renders through.
///
/// iced stores one pipeline per pipeline type and prepares every primitive
/// before rendering any, so two backgrounds drawn in the same frame would
/// overwrite each other's state unless they are distinct types. This marker is
/// the one difference between the page background and an overlay panel; the look
/// is otherwise configurable.
pub trait CompositeKind: 'static + Send + Sync + fmt::Debug {
    /// Names the GPU resources for debugging.
    const LABEL: &'static str;
}

/// The page background's pipeline instance.
#[derive(Debug, Clone, Copy)]
pub struct Background;

impl CompositeKind for Background {
    const LABEL: &'static str = "splash background";
}

/// An overlay panel's pipeline instance.
#[derive(Debug, Clone, Copy)]
pub struct Panel;

impl CompositeKind for Panel {
    const LABEL: &'static str = "splash panel";
}

/// A configurable frosted background, built by [`splash_background`] or
/// [`splash_panel`].
pub struct SplashBackground<K> {
    image: Option<Backdrop>,
    look: Look,
    kind: PhantomData<K>,
}

impl<K> SplashBackground<K> {
    /// Sets a flat base colour the surface settles into.
    pub fn base(mut self, base: Color) -> Self {
        self.look.base_top = base;
        self.look.base_bottom = base;
        self
    }

    /// Sets a vertical gradient base, eased from `top` to `bottom`.
    pub fn gradient(mut self, top: Color, bottom: Color) -> Self {
        self.look.base_top = top;
        self.look.base_bottom = bottom;
        self
    }

    /// Sets the blur radius, in source pixels.
    pub fn blur(mut self, blur: f32) -> Self {
        self.look.blur = blur;
        self
    }

    /// Sets the fraction down the surface by which the image wash has eased out.
    pub fn image_fade(mut self, fade: f32) -> Self {
        self.look.image_fade = fade;
        self
    }

    /// Sets the fraction down the surface by which the base color is fully in.
    pub fn settle(mut self, fraction: f32) -> Self {
        self.look.bg_end = fraction;
        self.look.bg_solid = fraction;
        self
    }

    /// Sets the vertical focal point for the cover-fit, 0 = top, 0.5 = centre.
    pub fn focus(mut self, focus: f32) -> Self {
        self.look.focus = focus;
        self
    }

    /// Sets the peak coverage of the image-less glow over the base.
    pub fn glow_strength(mut self, strength: f32) -> Self {
        self.look.glow_strength = strength;
        self
    }
}

impl<'a, Message, K> From<SplashBackground<K>> for Element<'a, Message>
where
    Message: 'a,
    K: CompositeKind,
{
    fn from(background: SplashBackground<K>) -> Self {
        Shader::new(shader::CompositeProgram::<K>::new(
            background.image,
            background.look,
        ))
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
    }
}
