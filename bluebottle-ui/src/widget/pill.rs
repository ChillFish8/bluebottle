use iced::widget::{Button, button, container, row, text};
use iced::{Background, Border, Center, Element, Padding, Theme};

use crate::{color, font, icon};

/// Create a new regular pill which can act as a button.
pub fn small<'a, Message>(label: &'a str, icon: Option<&'a str>) -> Button<'a, Message>
where
    Message: 'a,
{
    let label = text(label).size(12).line_height(1.0).font(font::regular());
    let icon = icon.map(|name| icon::filled(name).size(12));
    pill(label, icon)
}

/// Create a new regular pill which can act as a button.
pub fn regular<'a, Message>(label: &'a str, icon: Option<&'a str>) -> Button<'a, Message>
where
    Message: 'a,
{
    let label = text(label).size(14).line_height(1.0).font(font::regular());
    // We pad by +1 px here because the "Optical" center of the text on the horizonal axis
    // is one px higher than Iced will align it.
    let label_container = container(label).padding(Padding::default().bottom(1));
    let icon = icon.map(|name| icon::filled(name).size(14));
    pill(label_container, icon)
}

fn pill<'a, Message>(
    label: impl Into<Element<'a, Message>>,
    icon: Option<impl Into<Element<'a, Message>>>,
) -> Button<'a, Message>
where
    Message: 'a,
{
    let mut row = row![].align_y(Center).spacing(4);
    if let Some(icon) = icon {
        row = row.push(icon.into());
    }
    row = row.push(label);

    button(row)
        .padding(Padding::default().horizontal(8).vertical(4))
        .style(style)
}

fn style(_theme: &Theme, status: button::Status) -> button::Style {
    let text_color = match status {
        button::Status::Pressed => color::TEXT_PRIMARY,
        _ => color::TEXT_SECONDARY,
    };

    let background_color = match status {
        button::Status::Pressed => color::HOVER_HIGHLIGHT,
        button::Status::Hovered => color::HOVER_HIGHLIGHT,
        _ => color::SECONDARY,
    };

    button::Style {
        background: Some(Background::Color(background_color)),
        text_color,
        border: Border::default().rounded(28),
        shadow: Default::default(),
        snap: true,
    }
}
