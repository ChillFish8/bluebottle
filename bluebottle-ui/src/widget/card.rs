use iced::widget::{self, column, container, hover, text};
use iced::{Center, Element, Theme};

use super::button;
use crate::color::{TEXT_DEFAULT, TEXT_SECONDARY};

/// Creates a new widget that forms the core structure of the card button.
pub fn card<'a, Message>(
    label: &'a str,
    subtext: &'a str,
    display: impl Into<Element<'a, Message>>,
    overlay: impl Into<Element<'a, Message>>,
    on_click: Message,
) -> widget::Button<'a, Message>
where
    Message: 'a,
{
    let display = display.into();
    let overlay = overlay.into();

    let label = text(label).size(14).color(TEXT_DEFAULT);
    let subtext = text(subtext).size(12).color(TEXT_SECONDARY);

    let base = column![display, label, subtext].align_x(Center);

    // note: the padding is needed due to a clipping issue in the layout engine of iced (I think)
    widget::button(container(hover(base, overlay)).padding(1))
        .on_press(on_click)
        .style(wrapping_button_style)
}

fn wrapping_button_style(_theme: &Theme, _status: button::Status) -> button::Style {
    button::Style {
        background: None,
        text_color: Default::default(),
        border: Default::default(),
        shadow: Default::default(),
        snap: true,
    }
}
