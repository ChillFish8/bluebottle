use iced::widget::{Container, container, space};
use iced::{Background, Border, Length, Theme};

use crate::color;

/// A simple seperator line.
pub fn seperator<'a, Message>(width: Length) -> Container<'a, Message>
where
    Message: 'a,
{
    container(space().width(width).height(2)).style(default_style)
}

/// The default styling of the seperator line
pub fn default_style(_theme: &Theme) -> container::Style {
    container::Style {
        text_color: None,
        background: Some(Background::Color(color::HOVER_HIGHLIGHT)),
        border: Border::default().rounded(28),
        shadow: Default::default(),
        snap: true,
    }
}
