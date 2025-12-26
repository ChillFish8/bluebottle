use iced::{Element, border, widget};

use crate::color;

/// Wrap the element in a container with debug box lines.
pub fn container<'a, Message>(
    element: impl Into<Element<'a, Message>>,
) -> Element<'a, Message>
where
    Message: 'a,
{
    widget::container(element)
        .style(|_theme| widget::container::Style {
            text_color: None,
            background: None,
            border: border::Border::default().width(1).color(color::PRIMARY),
            shadow: Default::default(),
            snap: true,
        })
        .into()
}
