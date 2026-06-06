//! Bordered glass card. A base surface for grouped content. Wraps an iced
//! container with the bordered glass recipe used across buttons and dropdown
//! menus. The fill and ring are configurable so accented cards can swap in
//! the primary glass while keeping the same radius and ring weight.
//!
//! [`card`] returns a non-interactive container. [`clickable_card`] is the
//! interactive sibling. It wears the same chrome but routes hover and press
//! through the design system's [`clickable`](crate::widget::clickable)
//! chassis.

use iced::widget::{Container, container};
use iced::{Background, Border, Color, Element, Length, Padding, Theme};

use crate::color;
use crate::widget::clickable::clickable;

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

/// An interactive card chassis. The bordered glass recipe of [`card`] plus
/// hover and press affordances. Stays inert until `on_press` is supplied.
pub fn clickable_card<'a, Message>(
    child: impl Into<Element<'a, Message>>,
) -> ClickableCard<'a, Message>
where
    Message: Clone + 'a,
{
    ClickableCard {
        child: child.into(),
        background: color::with_alpha(color::WHITE, color::srgb_alpha(0.03)),
        border: color::border_strong(),
        radius: DEFAULT_RADIUS,
        padding: DEFAULT_PADDING,
        tint: color::hover_veil(),
        width: Length::Shrink,
        height: Length::Shrink,
        on_press: None,
    }
}

/// A bordered glass card that wraps `clickable`, built by [`clickable_card`].
pub struct ClickableCard<'a, Message> {
    child: Element<'a, Message>,
    background: Color,
    border: Color,
    radius: f32,
    padding: Padding,
    tint: Color,
    width: Length,
    height: Length,
    on_press: Option<Message>,
}

impl<'a, Message> ClickableCard<'a, Message>
where
    Message: Clone + 'a,
{
    /// Overrides the surface fill.
    pub fn background(mut self, color: Color) -> Self {
        self.background = color;
        self
    }

    /// Overrides the 1 px ring colour.
    pub fn border(mut self, color: Color) -> Self {
        self.border = color;
        self
    }

    /// Corner radius. Defaults to the card family's shared radius.
    pub fn radius(mut self, radius: f32) -> Self {
        self.radius = radius;
        self
    }

    /// Padding inside the card. Defaults to the same 4 px gutter as [`card`].
    pub fn padding(mut self, padding: impl Into<Padding>) -> Self {
        self.padding = padding.into();
        self
    }

    /// Hover-tint colour. Eased in over the surface on hover.
    pub fn tint(mut self, color: Color) -> Self {
        self.tint = color;
        self
    }

    /// Width of the card.
    pub fn width(mut self, width: impl Into<Length>) -> Self {
        self.width = width.into();
        self
    }

    /// Height of the card.
    pub fn height(mut self, height: impl Into<Length>) -> Self {
        self.height = height.into();
        self
    }

    /// Sets the press message. Enables the hover affordances and pointer
    /// cursor. Without one the card is inert.
    pub fn on_press(mut self, message: Message) -> Self {
        self.on_press = Some(message);
        self
    }

    /// Sets the press message from an [`Option`]. Mirrors the rest of the
    /// design system's optionally-interactive widgets.
    pub fn on_press_maybe(mut self, message: Option<Message>) -> Self {
        self.on_press = message;
        self
    }
}

impl<'a, Message> From<ClickableCard<'a, Message>> for Element<'a, Message>
where
    Message: Clone + 'a,
{
    fn from(card: ClickableCard<'a, Message>) -> Self {
        clickable(card.child)
            .background(card.background)
            .border(card.border)
            .radius(card.radius)
            .padding(card.padding)
            .tint(card.tint)
            .width(card.width)
            .height(card.height)
            .on_press_maybe(card.on_press)
            .into()
    }
}
