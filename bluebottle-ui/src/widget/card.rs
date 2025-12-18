use iced::widget::{self, column, container, hover, row, space, text};
use iced::{Center, Element, Length, Theme, border};

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

/// Creates a new skeleton loader for the card form, this can still take a separate display element
/// however, the element is not interactable.
pub fn skeleton<'a, Message>(
    display: impl Into<Element<'a, Message>>,
) -> Element<'a, Message>
where
    Message: Clone + 'a,
{
    let label = row![
        space().width(Length::FillPortion(1)),
        super::skeleton::skeleton()
            .height(12)
            .border(border::rounded(2))
            .width(Length::FillPortion(3)),
        space().width(Length::FillPortion(1)),
    ]
    .align_y(Center);

    let subtext = row![
        space().width(Length::FillPortion(2)),
        super::skeleton::skeleton()
            .height(10)
            .border(border::rounded(2))
            .width(Length::FillPortion(1)),
        space().width(Length::FillPortion(2)),
    ]
    .align_y(Center);

    let base = column![display.into(), label, subtext]
        .spacing(4)
        .align_x(Center);
    container(base).width(Length::Shrink).padding(1).into()
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
