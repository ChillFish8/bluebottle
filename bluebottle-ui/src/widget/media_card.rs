//! A media card with an image, optional label, and optional subtext.
//!
//! Only the image is clickable at the card level. Setting `.on_press(...)`
//! makes a click on the image publish that message. The label and subtext
//! are inert by default. They are optional Element slots so callers can
//! render any content, and if the caller wants them clickable they pass a
//! [`link`](super::link::link) element which carries its own press message
//! and hover underline. The image affordances (hover border, shadow, tint,
//! scaling overlay) live in [`media_image`](super::media_image::media_image).
//! This card is a thin shell that stacks a `media_image` with the optional
//! label and subtext rows below it.

use iced::widget::{column, container, row, space};
use iced::{Center, Element, Length};

use super::media_image::media_image;

/// Vertical gap between the image, label, and subtext.
const ROW_SPACING: f32 = 4.0;

/// Outer padding around the whole card. Leaves room for the focus border
/// and the drop shadow that paint just outside the image's bounds.
const CARD_PADDING: f32 = 2.0;

/// Creates a media card around `image`. The card is non-interactive by
/// default. Set `.on_press(...)` to make a click on the image publish a
/// message. Optional `.label(...)`, `.subtext(...)`, and `.overlay(...)`
/// extend the card. The label and subtext are inert unless the caller
/// passes a [`link`](super::link::link) element (or any other interactive
/// widget) in that slot.
pub fn media_card<'a, Message>(
    image: impl Into<Element<'a, Message>>,
) -> MediaCard<'a, Message>
where
    Message: Clone + 'a,
{
    MediaCard {
        image: image.into(),
        overlay: None,
        label: None,
        subtext: None,
        on_press: None,
    }
}

/// A configurable media card, built by [`media_card`].
pub struct MediaCard<'a, Message> {
    image: Element<'a, Message>,
    overlay: Option<Element<'a, Message>>,
    label: Option<Element<'a, Message>>,
    subtext: Option<Element<'a, Message>>,
    on_press: Option<Message>,
}

impl<'a, Message> MediaCard<'a, Message>
where
    Message: Clone + 'a,
{
    /// Adds a label row below the image.
    pub fn label(mut self, label: impl Into<Element<'a, Message>>) -> Self {
        self.label = Some(label.into());
        self
    }

    /// Adds a subtext row below the label.
    pub fn subtext(mut self, subtext: impl Into<Element<'a, Message>>) -> Self {
        self.subtext = Some(subtext.into());
        self
    }

    /// Layers `overlay` on top of the image, revealed by the hover
    /// animation when `on_press` is set.
    pub fn overlay(mut self, overlay: impl Into<Element<'a, Message>>) -> Self {
        self.overlay = Some(overlay.into());
        self
    }

    /// Sets the press message for the image. A click on the image
    /// publishes this message. Clicks on the label or subtext are inert
    /// unless the element in that slot is itself clickable (e.g. a
    /// [`link`](super::link::link)).
    pub fn on_press(mut self, message: Message) -> Self {
        self.on_press = Some(message);
        self
    }
}

impl<'a, Message> From<MediaCard<'a, Message>> for Element<'a, Message>
where
    Message: Clone + 'a,
{
    fn from(card: MediaCard<'a, Message>) -> Self {
        let mut image = media_image(card.image);
        if let Some(overlay) = card.overlay {
            image = image.overlay(overlay);
        }
        if let Some(on_press) = card.on_press {
            image = image.on_press(on_press);
        }

        let mut stack: Vec<Element<'a, Message>> = vec![image.into()];
        if let Some(label) = card.label {
            stack.push(label);
        }
        if let Some(subtext) = card.subtext {
            stack.push(subtext);
        }

        container(column(stack).spacing(ROW_SPACING))
            .padding(CARD_PADDING)
            .into()
    }
}

/// Creates a skeleton placeholder for a media card. The display element is
/// rendered as-is. `.label()` and `.subtext()` toggle stand-in shimmer rows
/// so the placeholder lines up with whichever rows the real card will show.
pub fn skeleton<'a, Message>(
    display: impl Into<Element<'a, Message>>,
) -> Skeleton<'a, Message>
where
    Message: Clone + 'a,
{
    Skeleton {
        display: display.into(),
        label: false,
        subtext: false,
    }
}

/// A configurable skeleton, built by [`skeleton`].
pub struct Skeleton<'a, Message> {
    display: Element<'a, Message>,
    label: bool,
    subtext: bool,
}

impl<'a, Message> Skeleton<'a, Message>
where
    Message: Clone + 'a,
{
    /// Shows a shimmer placeholder where the label row would sit.
    pub fn label(mut self) -> Self {
        self.label = true;
        self
    }

    /// Shows a shimmer placeholder where the subtext row would sit.
    pub fn subtext(mut self) -> Self {
        self.subtext = true;
        self
    }
}

impl<'a, Message> From<Skeleton<'a, Message>> for Element<'a, Message>
where
    Message: Clone + 'a,
{
    fn from(s: Skeleton<'a, Message>) -> Self {
        let label = row![
            super::skeleton::skeleton()
                .height(14)
                .radius(4.0)
                .width(Length::FillPortion(4)),
            space().width(Length::FillPortion(2)),
        ]
        .align_y(Center);

        let subtext = row![
            super::skeleton::skeleton()
                .height(12)
                .radius(4.0)
                .width(Length::FillPortion(2)),
            space().width(Length::FillPortion(2)),
        ]
        .align_y(Center);

        let mut base = column![s.display].spacing(ROW_SPACING);
        if s.label {
            base = base.push(label);
        }
        if s.subtext {
            base = base.push(subtext);
        }

        container(base).padding(CARD_PADDING).into()
    }
}
