use iced::Element;

use super::ICON_PADDING;
use crate::widget::clickable::clickable;
use crate::{color, icon};

/// An icon toggle button. The icon swaps when `selected` flips.
pub fn toggle_icon<'a, Message>(
    base_icon: &'a str,
    selected_icon: &'a str,
    selected: bool,
    message: Message,
) -> Element<'a, Message>
where
    Message: Clone + 'a,
{
    if selected {
        // Selected: icon sits at primary. The cascade drives the colour, so
        // the icon must not set an explicit `.color(...)`.
        clickable(icon::filled(selected_icon))
            .padding(ICON_PADDING)
            .resting_color(color::primary())
            .on_press(message)
            .into()
    } else {
        clickable(icon::outline(base_icon))
            .padding(ICON_PADDING)
            .on_press(message)
            .into()
    }
}
