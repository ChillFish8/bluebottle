use iced::widget::{button, column, container, hover, text};
use iced::{Background, Center, Element, Theme, border};

use crate::{color, icon};

/// A navbar button.
///
/// This is made up of icon + text with vertical alignment.
pub fn nav<'a, Message>(
    label: &'a str,
    icon: &'a str,
    selected: bool,
    message: Message,
) -> Element<'a, Message>
where
    Message: Clone + 'a,
{
    let nav_icon = |is_hovered: bool| -> Element<'a, Message> {
        container(container(icon::filled(icon)).padding([2, 16]))
            .style(move |_theme: &Theme| container::Style {
                background: is_hovered.then(|| Background::Color(color::HOVER_HIGHLIGHT)),
                text_color: selected.then(|| color::PRIMARY),
                border: border::rounded(28),
                ..Default::default()
            })
            .into()
    };

    fn style(theme: &Theme, status: button::Status) -> button::Style {
        let base = button::text(theme, status);

        let color = match status {
            button::Status::Pressed => color::TEXT_PRIMARY,
            _ => color::TEXT_DEFAULT,
        };

        button::Style {
            text_color: color,
            ..base
        }
    }

    let message = if selected { None } else { Some(message) };

    let base_button =
        button(column![nav_icon(false), text(label).size(14).style(text_primary)].align_x(Center))
            .style(style)
            .on_press_maybe(message.clone());

    let hovered_button =
        button(column![nav_icon(true), text(label).size(14).style(text_primary)].align_x(Center))
            .style(style)
            .on_press_maybe(message);

    if selected {
        hovered_button.into()
    } else {
        hover(base_button, hovered_button).into()
    }
}

fn text_primary(_theme: &Theme) -> text::Style {
    text::Style {
        color: Some(color::TEXT_DEFAULT),
    }
}