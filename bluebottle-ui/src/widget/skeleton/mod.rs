//! A shimmer skeleton placeholder.
//!
//! Every skeleton shares one shader pipeline and one global clock, so a page of
//! hundreds shows a single diagonal shimmer sweeping across all of them in
//! unison, at no per-box cost beyond a clipped quad.

mod shader;

use iced::widget::shader::Shader;
use iced::{Element, Length};

use self::shader::SkeletonProgram;
use crate::border;

/// A shimmer placeholder that fills its bounds as a rounded rectangle.
pub fn skeleton() -> Skeleton {
    Skeleton {
        width: Length::Fill,
        height: Length::Fill,
        radius: border::ROUNDED_MD,
    }
}

/// A configurable shimmer placeholder, built by [`skeleton`].
pub struct Skeleton {
    width: Length,
    height: Length,
    radius: f32,
}

impl Skeleton {
    /// Sets the width.
    pub fn width(mut self, width: impl Into<Length>) -> Self {
        self.width = width.into();
        self
    }

    /// Sets the height.
    pub fn height(mut self, height: impl Into<Length>) -> Self {
        self.height = height.into();
        self
    }

    /// Sets the corner radius. Half the short side gives a circle.
    pub fn radius(mut self, radius: f32) -> Self {
        self.radius = radius;
        self
    }
}

impl<'a, Message> From<Skeleton> for Element<'a, Message>
where
    Message: 'a,
{
    fn from(skeleton: Skeleton) -> Self {
        Shader::new(SkeletonProgram::new(skeleton.radius))
            .width(skeleton.width)
            .height(skeleton.height)
            .into()
    }
}
