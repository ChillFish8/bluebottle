use iced::widget::{container, row, text};
use iced::{Center, Color, Element, padding};

use crate::widget::clickable::clickable;
use crate::{color, font, icon};

/// The brightest thing on a dark, blurred stage.
///
/// No more than one per surface.
pub fn hero<'a, Message>(
    icon_name: &'a str,
    label: &'a str,
    message: Message,
) -> Element<'a, Message>
where
    Message: Clone + 'a,
{
    hero_inner(icon_name, label, color::WHITE, color::BG, 0.06, message)
}

/// The same as [hero], but with a primary fill.
pub fn hero_primary<'a, Message>(
    icon_name: &'a str,
    label: &'a str,
    message: Message,
) -> Element<'a, Message>
where
    Message: Clone + 'a,
{
    hero_inner(
        icon_name,
        label,
        color::primary(),
        color::WHITE,
        0.10,
        message,
    )
}

fn hero_inner<'a, Message>(
    icon_name: &'a str,
    label: &'a str,
    fill: Color,
    text_color: Color,
    glow_alpha: f32,
    message: Message,
) -> Element<'a, Message>
where
    Message: Clone + 'a,
{
    let label = text(label).size(font::TEXT_MEDIUM).font(font::bold());

    let items = row![
        icon::filled(icon_name).size(16),
        container(label).padding(padding::bottom(2))
    ]
    .spacing(4)
    .align_y(Center);

    let padding = padding::Padding::default().horizontal(22).vertical(10);

    // The fill carries the colour and the hover tint is disabled, so only
    // the glow reacts on hover.
    clickable(items)
        .padding(padding)
        .background(fill)
        .glow(glow_alpha)
        .resting_color(text_color)
        .tint(Color::TRANSPARENT)
        .on_press(message)
        .into()
}
