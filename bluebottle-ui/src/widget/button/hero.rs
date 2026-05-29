use iced::widget::{button, container, row, text};
use iced::{Background, Center, Theme, padding};

use crate::button::Status;
use crate::{border, color, font, icon};

/// The brightest thing on a dark, blurred stage.
///
/// No more than one per surface.
pub fn hero<'a, Message>(
    icon_name: &'a str,
    label: &'a str,
    message: Message,
) -> button::Button<'a, Message>
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

    button(items)
        .on_press(message)
        .style(hero_styling_default)
        .padding(padding)
}

/// The same as [hero], but with a primary fill.
pub fn hero_primary<'a, Message>(
    icon_name: &'a str,
    label: &'a str,
    message: Message,
) -> button::Button<'a, Message>
where
    Message: Clone + 'a,
{
    hero(icon_name, label, message).style(|theme: &Theme, status: Status| {
        let mut style = hero_styling_default(theme, status);
        style.text_color = color::WHITE;
        style.background = Some(Background::Color(color::primary()));
        style
    })
}

fn hero_styling_default(_theme: &Theme, _status: Status) -> button::Style {
    button::Style {
        text_color: color::BG,
        background: Some(Background::Color(color::WHITE)),
        border: border::rounded(border::ROUNDED_FULL),
        ..button::Style::default()
    }
}
