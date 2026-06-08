use iced::widget::{button, row, text};
use iced::{Center, Element, Theme};

use super::{Status, Style};
use crate::{border, color, icon, spacing};

/// Padding around a disabled icon so it slots into the same rows as a standard
/// button without shifting.
const ICON_PADDING: u16 = 4;

/// A disabled button. Cannot be interacted with. Sizes like a
/// [`standard`](super::standard) or [`icon`](super::icon) button so it slots
/// into the same rows without shifting.
pub fn disabled<'a, Message>(
    label: Option<&'a str>,
    icon_name: Option<&'a str>,
) -> Element<'a, Message>
where
    Message: Clone + 'a,
{
    if let (Some(name), None) = (icon_name, label) {
        let inner = icon::filled(name).size(24).color(color::TEXT_DARK);
        return button(inner)
            .padding(ICON_PADDING)
            .style(disabled_button_style)
            .into();
    }

    let mut items = row![].spacing(spacing::GAP_4).align_y(Center);

    if let Some(name) = icon_name {
        items = items.push(icon::filled(name).size(24).color(color::TEXT_DARK));
    }

    if let Some(label) = label {
        items = items.push(text(label).color(color::TEXT_DARK));
    }

    button(items).style(disabled_button_style).into()
}

fn disabled_button_style(_theme: &Theme, _status: Status) -> Style {
    Style {
        text_color: color::TEXT_DARK,
        background: None,
        border: border::rounded(border::ROUNDED_FULL),
        ..Style::default()
    }
}
