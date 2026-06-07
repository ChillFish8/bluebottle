use iced::widget::container;
use iced::{Center, Element, Length};

use crate::icon;

/// Centred glyph in a fixed-size circle. Shared chassis for the round
/// icon button variants. Returns the inner content. The caller wraps it
/// with [`clickable`](crate::widget::clickable::clickable) to add the
/// glass, press behaviour, and pill border.
pub(super) fn icon_circle<'a, Message>(
    icon_name: &'a str,
    diameter: f32,
    icon_size: f32,
) -> Element<'a, Message>
where
    Message: 'a,
{
    let diameter = Length::Fixed(diameter);
    container(icon::filled(icon_name).size(icon_size))
        .width(diameter)
        .height(diameter)
        .align_x(Center)
        .align_y(Center)
        .into()
}
