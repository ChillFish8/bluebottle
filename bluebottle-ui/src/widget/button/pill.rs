use iced::{padding, Center, Element};
use iced::widget::{row, text};

use crate::{clickable, icon};




/// Ghost Pill
///
/// The ghost is a one-shot action, never a selected state.
pub fn ghost<'a, Message>(
    label: &'a str,
    icon_name: Option<&'a str>,
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

    let padding = padding::Padding::default().horizontal(16).vertical(8);

    todo!()
}

/// Ghost Pill Small Variant
///
/// The ghost is a one-shot action, never a selected state.
pub fn ghost_small<'a, Message>(
    label: &'a str,
    icon_name: Option<&'a str>,
    message: Message,
) -> Element<'a, Message>
where
    Message: Clone + 'a,
{
    let mut items = row![].spacing(4).align_y(Center);
    if let Some(name) = icon_name {
        items = items.push(icon::filled(name).size(11));
    }
    items = items.push(text(label).size(11));

    let padding = padding::Padding::default().horizontal(12).vertical(6);

    clickable(items)
        .padding(padding)
}

/// Ghost Pill
///
/// The ghost is a one-shot action, never a selected state.
pub fn toggle<'a, Message>(
    label: &'a str,
    icon_name: Option<&'a str>,
    message: Message,
) -> Element<'a, Message>
where
    Message: Clone + 'a,
{
    todo!()
}
