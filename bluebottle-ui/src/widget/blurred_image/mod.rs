//! An image widget that frosts declared regions of itself behind an overlay.
//!
//! The image renders sharp by default. Each [`Rectangle`] passed to
//! [`BlurredImage::region`] (or [`BlurredImage::regions`]) is composited from
//! a blurred copy of the same image, with the supplied corner radius. The
//! blur runs once over the whole image and the rounded-rect mask discriminates
//! which pixels read from it.
//!
//! Pair it with [`overlay`](BlurredImage::overlay) to layer interactive
//! widgets on top. The overlay sits above the composited image and can stay
//! transparent where the user wants the sharp image to read through. Non-
//! transparent surfaces inside the overlay are what the blur regions should
//! correspond to.
//!
//! Regions are in widget-local logical pixels. For a widget at a known fixed
//! size, [`region`](BlurredImage::region) and [`regions`](BlurredImage::regions)
//! are the simple path. For [`Length::Fill`] widgets whose laid-out size is
//! only known at draw time, use [`regions_fn`](BlurredImage::regions_fn) to
//! compute the rects from the size each frame.

mod shader;

use std::sync::Arc;

use iced::widget::shader::Shader;
use iced::widget::stack;
use iced::{Element, Length, Rectangle, Size};
pub use shader::MAX_REGIONS;

use crate::style;
use crate::widget::blur::Backdrop;
use crate::widget::skeleton::DEFAULT_RADIUS;

/// Creates a blurred image over `backdrop`.
///
/// Without any [`region`](BlurredImage::region), the widget renders the image
/// sharp and untouched. Adding regions enables the frosted composite.
pub fn blurred_image<'a, Message>(backdrop: Backdrop) -> BlurredImage<'a, Message> {
    BlurredImage {
        backdrop,
        overlay: None,
        regions: Vec::new(),
        regions_fn: None,
        blur_radius: style::IMAGE_BLUR,
        corner_radius: DEFAULT_RADIUS,
        width: Length::Fill,
        height: Length::Fill,
    }
}

/// A configurable blurred-region image, built by [`blurred_image`].
pub struct BlurredImage<'a, Message> {
    backdrop: Backdrop,
    overlay: Option<Element<'a, Message>>,
    regions: Vec<Rectangle>,
    regions_fn: Option<Arc<dyn Fn(Size) -> Vec<Rectangle> + Send + Sync>>,
    blur_radius: f32,
    corner_radius: f32,
    width: Length,
    height: Length,
}

impl<'a, Message> BlurredImage<'a, Message> {
    /// Layers `overlay` on top of the composited image.
    pub fn overlay(mut self, overlay: impl Into<Element<'a, Message>>) -> Self {
        self.overlay = Some(overlay.into());
        self
    }

    /// Adds one frosted region, in widget-local logical pixels.
    ///
    /// Has no effect once [`regions_fn`](Self::regions_fn) is set, since the
    /// closure is the source of truth in that mode.
    pub fn region(mut self, region: Rectangle) -> Self {
        self.regions.push(region);
        self
    }

    /// Extends the frosted regions from any iterator of rectangles.
    ///
    /// Has no effect once [`regions_fn`](Self::regions_fn) is set.
    pub fn regions(mut self, regions: impl IntoIterator<Item = Rectangle>) -> Self {
        self.regions.extend(regions);
        self
    }

    /// Derives the frosted regions from the laid-out widget size each frame.
    ///
    /// Use this when the widget itself is `Length::Fill` (or otherwise sized
    /// by its parent) and the region rects need to follow. Replaces any
    /// regions previously added through [`region`](Self::region) or
    /// [`regions`](Self::regions).
    pub fn regions_fn<F>(mut self, f: F) -> Self
    where
        F: Fn(Size) -> Vec<Rectangle> + Send + Sync + 'static,
    {
        self.regions_fn = Some(Arc::new(f));
        self
    }

    /// Overrides the blur radius, in source pixels. Default tracks
    /// [`style::IMAGE_BLUR`](crate::style::IMAGE_BLUR).
    pub fn blur(mut self, radius: f32) -> Self {
        self.blur_radius = radius;
        self
    }

    /// Overrides the corner radius applied to each region.
    pub fn corner_radius(mut self, radius: f32) -> Self {
        self.corner_radius = radius;
        self
    }

    pub fn width(mut self, width: impl Into<Length>) -> Self {
        self.width = width.into();
        self
    }

    pub fn height(mut self, height: impl Into<Length>) -> Self {
        self.height = height.into();
        self
    }
}

impl<'a, Message> From<BlurredImage<'a, Message>> for Element<'a, Message>
where
    Message: 'a,
{
    fn from(image: BlurredImage<'a, Message>) -> Self {
        let region_source = match image.regions_fn {
            Some(f) => shader::RegionSource::Dynamic(f),
            None => shader::RegionSource::Static(Arc::from(image.regions)),
        };
        let composite = Shader::new(shader::BlurredImageProgram::new(
            image.backdrop,
            image.blur_radius,
            image.corner_radius,
            region_source,
        ))
        .width(image.width)
        .height(image.height);

        match image.overlay {
            Some(overlay) => stack![composite, overlay].into(),
            None => composite.into(),
        }
    }
}
