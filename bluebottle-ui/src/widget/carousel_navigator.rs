use iced::widget::{row, text};
use iced::{Center, Element};

use crate::{button, color, font, icon};

/// The carousel navigator for switching pages.
pub fn navigator<'a, Message>(
    current_page: u32,
    total_pages: u32,
    on_back: Message,
    on_forward: Message,
) -> Element<'a, Message>
where
    Message: Clone + 'a,
{
    let on_back = (current_page > 1).then_some(on_back);
    let on_forward = (current_page < total_pages).then_some(on_forward);

    let label = text(format!("{current_page} / {total_pages}"))
        .font(font::regular())
        .color(color::TEXT_SECONDARY)
        .size(14)
        .width(32)
        .align_x(Center);

    row![
        maybe_disabled_button("chevron_left", on_back),
        label,
        maybe_disabled_button("chevron_right", on_forward),
    ]
    .align_y(Center)
    .spacing(4)
    .padding(4)
    .into()
}

fn maybe_disabled_button<'a, Message>(
    icon: &'a str,
    message: Option<Message>,
) -> Element<'a, Message>
where
    Message: Clone + 'a,
{
    if let Some(msg) = message {
        button::icon(icon, false, msg).into()
    } else {
        button::disabled(None, Some(icon))
    }
}
