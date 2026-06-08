use iced::widget::row;
use iced::{Center, Element};

use crate::{button, font, spacing, text};

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

    let label = text::caption(format!("{current_page} / {total_pages}"))
        .font(font::bold())
        .width(32)
        .align_x(Center);

    row![
        button::icon_carousel("chevron_left", on_back),
        label,
        button::icon_carousel("chevron_right", on_forward),
    ]
    .align_y(Center)
    .spacing(spacing::GAP_4)
    .padding(spacing::PAD_4)
    .into()
}
