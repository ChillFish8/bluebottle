use iced::widget::text_input;
use iced::{Background, Border, Element, Theme, padding};

use crate::color;

/// Creates a new search input widget which triggers an event when ever the user types.
pub fn search<'a, Message>(
    placeholder: &'a str,
    content: &'a str,
    on_input: impl Fn(String) -> Message + 'a,
) -> Element<'a, Message>
where
    Message: Clone + 'a,
{
    text_input(placeholder, content)
        .style(input_style)
        .on_input(on_input)
        .padding(padding::Padding::default().horizontal(16).vertical(8))
        .into()
}

fn input_style(_theme: &Theme, status: text_input::Status) -> text_input::Style {
    let background_color = match status {
        text_input::Status::Hovered => color::HOVER_HIGHLIGHT,
        text_input::Status::Focused { .. } => color::HOVER_HIGHLIGHT,
        _ => color::SECONDARY,
    };

    let border_color = match status {
        text_input::Status::Focused { .. } => color::PRIMARY,
        _ => color::HOVER_HIGHLIGHT,
    };

    text_input::Style {
        background: Background::Color(background_color),
        border: Border::default().rounded(28).color(border_color).width(1),
        icon: Default::default(),
        placeholder: color::TEXT_DARK,
        value: color::TEXT_SECONDARY,
        selection: color::TEXT_DARK,
    }
}
