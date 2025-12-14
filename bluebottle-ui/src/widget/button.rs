use iced::widget::{button, column, container, hover, row, text};
use iced::{Background, Center, Color, Element, Theme, border};

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
                background: is_hovered
                    .then(|| Background::Color(color::HOVER_HIGHLIGHT)),
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

    let message = (!selected).then_some(message);

    let base_button = button(
        column![
            nav_icon(false),
            text(label).size(12).style(text_forced_default)
        ]
        .align_x(Center),
    )
    .style(style)
    .on_press_maybe(message.clone());

    let hovered_button = button(
        column![
            nav_icon(true),
            text(label).size(12).style(text_forced_default)
        ]
        .align_x(Center),
    )
    .style(style)
    .on_press_maybe(message);

    if selected {
        hovered_button.into()
    } else {
        hover(base_button, hovered_button).into()
    }
}

/// A standard button
///
/// This is made up of an optional icon + text with horizontal alignment.
pub fn standard<'a, Message>(
    label: &'a str,
    icon: Option<&'a str>,
    selected: bool,
    message: Message,
) -> Element<'a, Message>
where
    Message: Clone + 'a,
{
    let mut items = row![].spacing(4).align_y(Center);
    if let Some(icon) = icon {
        items = items.push(icon::filled(icon).size(24));
    }
    items = items.push(text(label));

    let message = (!selected).then_some(message);
    button(items)
        .style(default_button_style)
        .on_press_maybe(message)
        .into()
}

/// An icon button
///
/// This has no label, only a clickable icon.
pub fn icon<'a, Message>(
    icon: &'a str,
    selected: bool,
    message: Message,
) -> Element<'a, Message>
where
    Message: Clone + 'a,
{
    let message = (!selected).then_some(message);
    button(icon::filled(icon))
        .padding(4)
        .style(default_button_style)
        .on_press_maybe(message)
        .into()
}

/// An icon toggle button
///
/// This has no label, only a clickable icon which becomes filled when clicked/selected.
pub fn toggle_icon<'a, Message>(
    base_icon: &'a str,
    selected_icon: &'a str,
    selected: bool,
    message: Message,
) -> Element<'a, Message>
where
    Message: Clone + 'a,
{
    let style = move |theme: &Theme, status: button::Status| {
        let mut base = default_button_style(theme, status);
        if selected {
            base.text_color = color::TEXT_PRIMARY;
        }
        base
    };

    let icon = if selected {
        icon::filled(selected_icon)
    } else {
        icon::outline(base_icon)
    };

    button(icon)
        .padding(4)
        .style(style)
        .on_press(message)
        .into()
}

fn default_button_style(theme: &Theme, status: button::Status) -> button::Style {
    let base = button::text(theme, status);

    let color = match status {
        button::Status::Pressed => color::TEXT_PRIMARY,
        button::Status::Disabled => color::TEXT_PRIMARY,
        _ => color::TEXT_DEFAULT,
    };

    let background = match status {
        button::Status::Hovered => color::HOVER_HIGHLIGHT,
        button::Status::Pressed => color::HOVER_HIGHLIGHT,
        _ => Color::TRANSPARENT,
    };

    button::Style {
        text_color: color,
        background: Some(Background::Color(background)),
        border: border::rounded(999),
        ..base
    }
}

fn text_forced_default(_theme: &Theme) -> text::Style {
    text::Style {
        color: Some(color::TEXT_DEFAULT),
    }
}
