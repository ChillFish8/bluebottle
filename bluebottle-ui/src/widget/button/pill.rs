use iced::widget::row;
use iced::{Center, Color, Element, padding};

use crate::widget::clickable::clickable;
use crate::{color, icon, text};

/// Ghost Pill
///
/// A rank-two, one-shot action. A white label with an optional icon,
/// transparent at rest, borrowing the neutral glass look on hover. Accent is
/// kept out on purpose, the ghost is never a selected state.
pub fn ghost<'a, Message>(
    label: &'a str,
    icon_name: Option<&'a str>,
    message: Message,
) -> Element<'a, Message>
where
    Message: Clone + 'a,
{
    let mut items = row![].spacing(4).align_y(Center);
    if let Some(name) = icon_name {
        items = items.push(icon::filled(name).size(16));
    }
    items = items.push(text::label(label, text::Variant::Main));

    let pad = padding::Padding::default().horizontal(16).vertical(8);
    ghost_pill(items, pad, color::TEXT_PRIMARY, message)
}

/// Ghost Pill Small Variant
///
/// The same one-shot action at the Caption role with a dimmer tone and a
/// tighter pad.
pub fn ghost_small<'a, Message>(
    label: &'a str,
    icon_name: Option<&'a str>,
    message: Message,
) -> Element<'a, Message>
where
    Message: Clone + 'a,
{
    let mut items = row![].spacing(4).align_y(Center);
    if let Some(name) = icon_name {
        items = items.push(icon::filled(name).size(14));
    }
    items = items.push(text::caption(label));

    let pad = padding::Padding::default().horizontal(12).vertical(6);
    ghost_pill(items, pad, color::TEXT_SECONDARY, message)
}

/// Shared chassis for both ghost sizes. The hover glass is identical. Only the
/// content, pad, and resting tone differ.
fn ghost_pill<'a, Message>(
    items: impl Into<Element<'a, Message>>,
    pad: padding::Padding,
    tone: Color,
    message: Message,
) -> Element<'a, Message>
where
    Message: Clone + 'a,
{
    // Neutral glass on hover. A white 10% fill behind a white 10% hairline.
    let glass = color::border_strong();

    clickable(items.into())
        .padding(pad)
        .tint(glass)
        .hover_border(glass)
        .resting_color(tone)
        .on_press(message)
        .into()
}
