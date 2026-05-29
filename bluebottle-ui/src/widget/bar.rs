use iced::widget::{column, space};
use iced::{Center, Element, Length, padding};

/// Create a sidebar.
pub fn side<'a, Message>(
    top: impl Into<Element<'a, Message>>,
    bottom: impl Into<Element<'a, Message>>,
) -> Element<'a, Message>
where
    Message: 'a,
{
    column![top.into(), space().height(Length::Fill), bottom.into()]
        .width(80)
        .align_x(Center)
        .padding(padding::Padding::default().vertical(8))
        .into()
}
