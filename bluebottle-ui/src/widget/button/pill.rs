use iced::widget::text::IntoFragment;
use iced::widget::{container, row};
use iced::{Center, Color, Element, padding};

use crate::widget::clickable::clickable;
use crate::{color, font, icon, text};

/// Ghost Pill
///
/// A rank-two, one-shot action. A white label with an optional icon,
/// transparent at rest, borrowing the neutral glass look on hover. Accent is
/// kept out on purpose, the ghost is never a selected state.
pub fn ghost<'a, Message>(
    label: impl IntoFragment<'a>,
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
    label: impl IntoFragment<'a>,
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

/// Toggle Pill
///
/// A stateful toolbar toggle on the ghost chassis. Transparent at rest, the
/// same neutral glass on hover, and an accent recipe when `on`. An accent 28%
/// fill behind a full accent ring with the label and icon tinted accent.
pub fn toggle_pill<'a, Message>(
    label: impl IntoFragment<'a>,
    icon_name: Option<&'a str>,
    on: bool,
    message: Message,
) -> Element<'a, Message>
where
    Message: Clone + 'a,
{
    // The label defers its colour so the clickable can ease it between the
    // resting white and the accent on tone. The icon already defers.
    let optically_padded = container(
        text::label(label, text::Variant::Main)
            .font(font::medium())
            .inherit_color(),
    )
    .padding(padding::bottom(1));

    let mut items = row![].spacing(8).align_y(Center);
    if let Some(name) = icon_name {
        items = items.push(icon::filled(name).size(14));
    }
    items = items.push(optically_padded);

    let pad = padding::Padding::default().horizontal(14).vertical(7);
    let glass = color::border_strong();

    // Off is the ghost glass on hover. On is the accent recipe, an accent 28%
    // fill behind a full accent ring with accent label and icon. The clickable
    // crossfades between them, the glass receding as the accent fades in.
    clickable(items)
        .padding(pad)
        .resting_color(color::TEXT_PRIMARY)
        .tint(glass)
        .hover_border(glass)
        .selected(on)
        .selected_background(color::primary_glass())
        .selected_border(color::primary())
        .selected_color(color::primary())
        .on_press(message)
        .into()
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
