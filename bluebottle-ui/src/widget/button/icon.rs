use iced::Element;
use iced::widget::Text;

use super::ICON_PADDING;
use crate::widget::clickable::clickable;
use crate::{color, icon};

#[doc(hidden)]
/// An icon name or pre-created icon text widget.
pub enum IconTextOrName<'a> {
    Name(&'a str),
    Text(Text<'a>),
}

impl<'a> From<&'a str> for IconTextOrName<'a> {
    fn from(value: &'a str) -> Self {
        Self::Name(value)
    }
}

impl<'a> From<Text<'a>> for IconTextOrName<'a> {
    fn from(value: Text<'a>) -> Self {
        Self::Text(value)
    }
}

/// An icon button. No label, only a clickable icon.
pub fn icon<'a, Message>(
    icon_input: impl Into<IconTextOrName<'a>>,
    selected: bool,
    message: Message,
) -> Element<'a, Message>
where
    Message: Clone + 'a,
{
    let inner = match icon_input.into() {
        IconTextOrName::Name(name) => icon::filled(name),
        IconTextOrName::Text(text) => text,
    };

    let message = (!selected).then_some(message);
    let mut button = clickable(inner)
        .padding(ICON_PADDING)
        .on_press_maybe(message);
    if selected {
        button = button.resting_color(color::primary());
    }
    button.into()
}
