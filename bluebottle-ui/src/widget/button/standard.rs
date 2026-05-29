use iced::widget::{row, text};
use iced::{Center, Element};

use crate::widget::clickable::clickable;
use crate::{color, icon};

const STANDARD_PADDING: [u16; 2] = [5, 10];

/// A standard button. Optional leading icon plus label, pill background.
pub fn standard<'a, Message>(
    label: &'a str,
    icon_name: Option<&'a str>,
    selected: bool,
    message: Message,
) -> Element<'a, Message>
where
    Message: Clone + 'a,
{
    let mut items = row![].spacing(4).align_y(Center);
    if let Some(name) = icon_name {
        items = items.push(icon::filled(name).size(24));
    }
    items = items.push(text(label));

    let message = (!selected).then_some(message);
    let mut button = clickable(items)
        .padding(STANDARD_PADDING)
        .on_press_maybe(message);
    if selected {
        button = button.resting_color(color::primary());
    }
    button.into()
}
