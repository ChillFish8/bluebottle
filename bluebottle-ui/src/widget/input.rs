use iced::{Background, Border, Theme, padding, widget};

use crate::color;

/// Creates a new text input widget that is notified on user typing.
pub fn text_input<'a, Message>(
    placeholder: &'a str,
    content: &'a str,
    on_input: impl Fn(String) -> Message + 'a,
) -> widget::text_input::TextInput<'a, Message>
where
    Message: Clone + 'a,
{
    iced::widget::text_input(placeholder, content)
        .style(input_style)
        .on_input(on_input)
        .padding(padding::Padding::default().horizontal(16).vertical(8))
}

fn input_style(
    _theme: &Theme,
    status: widget::text_input::Status,
) -> widget::text_input::Style {
    let background_color = match status {
        widget::text_input::Status::Hovered => color::HOVER_HIGHLIGHT,
        widget::text_input::Status::Focused { .. } => color::HOVER_HIGHLIGHT,
        _ => color::SECONDARY,
    };

    let border_color = match status {
        widget::text_input::Status::Focused { .. } => color::PRIMARY,
        _ => color::HOVER_HIGHLIGHT,
    };

    widget::text_input::Style {
        background: Background::Color(background_color),
        border: Border::default().rounded(28).color(border_color).width(1),
        icon: Default::default(),
        placeholder: color::TEXT_DARK,
        value: color::TEXT_SECONDARY,
        selection: color::TEXT_DARK,
    }
}
