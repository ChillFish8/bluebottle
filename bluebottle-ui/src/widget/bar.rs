use iced::widget::{column, row, space, text};
use iced::{Center, Element, Length, padding};

use crate::{color, font};

/// Create a topbar.
pub fn top<'a, Message>(
    center: impl Into<Element<'a, Message>>,
    active_library: &'a str,
) -> Element<'a, Message>
where
    Message: 'a,
{
    let title = text("Bluebottle").size(18).font(font::semibold());
    let active_library = text(active_library)
        .size(16)
        .font(font::semibold())
        .color(color::TEXT_SECONDARY);

    row![
        title,
        space().width(Length::Fill),
        center.into(),
        space().width(Length::Fill),
        active_library
    ]
    .height(32)
    .padding(padding::Padding::default().vertical(4).horizontal(8))
    .align_y(Center)
    .into()
}

/// Create a sidebar.
pub fn side<'a, Message>(
    top: impl Into<Element<'a, Message>>,
    bottom: impl Into<Element<'a, Message>>,
) -> Element<'a, Message>
where
    Message: 'a,
{
    column![top.into(), space().height(Length::Fill), bottom.into()]
        .align_x(Center)
        .padding(
            padding::Padding::default()
                .horizontal(4)
                .vertical(8)
        )
        .into()
}
