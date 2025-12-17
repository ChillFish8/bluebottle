use iced::widget::{row, text};
use iced::{Center, Element};

use crate::{color, font, icon};

/// A title widget with optional icon.
pub fn title<'a, Message>(icon: Option<&'a str>, title: &'a str) -> Element<'a, Message>
where
    Message: 'a,
{
    let title = text(title)
        .size(20)
        .font(font::semibold())
        .color(color::TEXT_DEFAULT);

    if let Some(icon) = icon {
        let icon = icon::filled(icon);
        row![icon, title].spacing(4).align_y(Center).into()
    } else {
        title.into()
    }
}
