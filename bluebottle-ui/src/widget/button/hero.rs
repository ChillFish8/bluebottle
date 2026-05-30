use iced::widget::{container, row};
use iced::{Center, Color, Element, padding};

use crate::widget::clickable::clickable;
use crate::{color, icon, text};

/// The brightest thing on a dark, blurred stage. A white fill with dark text
/// and a soft glow.
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
    let label = text::hero_label(label);

    let items = row![
        icon::filled(icon_name).size(16),
        container(label).padding(padding::bottom(2))
    ]
    .spacing(4)
    .align_y(Center);

    let padding = padding::Padding::default().horizontal(22).vertical(10);

    // The white fill carries the colour and the hover tint is disabled, so
    // only the glow reacts on hover.
    clickable(items)
        .padding(padding)
        .background(color::WHITE)
        .glow()
        .resting_color(color::BG)
        .tint(Color::TRANSPARENT)
        .on_press(message)
        .into()
}
