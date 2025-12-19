use iced::widget::{container, space};
use iced::{Background, Border, Element, Length};

use crate::color;

/// A simple seperator line.
pub fn seperator<'a, Message>(width: Length) -> Element<'a, Message>
where
    Message: 'a,
{
    container(space().width(width).height(2))
        .style(|_| container::Style {
            text_color: None,
            background: Some(Background::Color(color::HOVER_HIGHLIGHT)),
            border: Border::default().rounded(28),
            shadow: Default::default(),
            snap: true,
        })
        .into()
}
