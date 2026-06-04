//! Bordered glass card. A base surface for grouped content. Wraps an iced
//! container with the bordered glass recipe used across buttons and dropdown
//! menus. The fill and ring are configurable so accented cards can swap in
//! the primary glass while keeping the same radius and ring weight.

use iced::widget::{Container, container};
use iced::{Background, Border, Color, Element, Length, Padding, Theme};

use crate::color;

const DEFAULT_RADIUS: f32 = 14.0;
const DEFAULT_PADDING: Padding = Padding {
    top: 4.0,
    right: 4.0,
    bottom: 4.0,
    left: 4.0,
};

/// A bordered glass card around `child`.
///
/// Defaults to the neutral button glass with a 1 px ring. Chain
/// [`Card::background`] and [`Card::border`] for accented variants.
pub fn card<'a, Message>(child: impl Into<Element<'a, Message>>) -> Card<'a, Message>
where
    Message: 'a,
{
    Card {
        inner: container(child).padding(DEFAULT_PADDING),
        background: color::with_alpha(color::WHITE, color::srgb_alpha(0.03)),
        border: color::border_strong(),
        radius: DEFAULT_RADIUS,
    }
}

/// A bordered glass surface built by [`card`].
pub struct Card<'a, Message> {
    inner: Container<'a, Message>,
    background: Color,
    border: Color,
    radius: f32,
}

impl<'a, Message> Card<'a, Message>
where
    Message: 'a,
{
    /// Overrides the surface fill. Pair with [`Card::border`] for accent
    /// variants.
    pub fn background(mut self, color: Color) -> Self {
        self.background = color;
        self
    }

    /// Overrides the 1 px ring colour.
    pub fn border(mut self, color: Color) -> Self {
        self.border = color;
        self
    }

    /// Corner radius. Defaults to the dropdown menu radius.
    pub fn radius(mut self, radius: f32) -> Self {
        self.radius = radius;
        self
    }

    /// Padding inside the card. Defaults to 4 px on every edge.
    pub fn padding(mut self, padding: impl Into<Padding>) -> Self {
        self.inner = self.inner.padding(padding);
        self
    }

    /// Width of the card.
    pub fn width(mut self, width: impl Into<Length>) -> Self {
        self.inner = self.inner.width(width);
        self
    }

    /// Height of the card.
    pub fn height(mut self, height: impl Into<Length>) -> Self {
        self.inner = self.inner.height(height);
        self
    }
}

impl<'a, Message> From<Card<'a, Message>> for Element<'a, Message>
where
    Message: 'a,
{
    fn from(card: Card<'a, Message>) -> Self {
        let background = card.background;
        let border = card.border;
        let radius = card.radius;

        card.inner
            .style(move |_theme: &Theme| container::Style {
                background: Some(Background::Color(background)),
                border: Border {
                    radius: radius.into(),
                    width: 1.0,
                    color: border,
                },
                ..container::Style::default()
            })
            .into()
    }
}
