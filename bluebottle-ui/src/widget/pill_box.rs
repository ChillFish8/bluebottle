use iced::Element;
use iced::widget::{column, row, text};

use crate::{color, font};

/// Creates a new box for holding a set of pills
pub fn pill_box<'a, Message>(
    label: &'a str,
    pills: impl IntoIterator<Item = Element<'a, Message>>,
) -> Element<'a, Message>
where
    Message: 'a,
{
    let label = text(label)
        .size(14)
        .color(color::TEXT_DARK)
        .font(font::regular());

    let grid = row(pills).spacing(4).wrap();

    column![label, grid,].spacing(4).into()
}
