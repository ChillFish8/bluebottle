//! Toggle Row
//!
//! The switch served as a list item. A label and optional sub-line on the
//! left, a 40 px bordered glass switch on the right. The whole capsule is
//! the hit target, so a generous swipe at the row toggles the setting the
//! same way a precise tap on the switch does.
//!
//! The row chrome stays transparent at rest and at on. Hover veils to
//! white 6 %. The switch alone carries the accent, so a column of enabled
//! settings does not flood with indigo. Mirrors the dropdown filter row,
//! with a switch in place of the checkbox for instant-apply settings.

use iced::widget::{Column, Row, Space};
use iced::{Element, Length, Padding, alignment};

use super::switch::{SwitchSizeVariant, switch};
use crate::widget::clickable::clickable;
use crate::widget::text;
use crate::{border, color, font, spacing};

const ROW_PADDING: Padding = Padding {
    top: spacing::PAD_8,
    right: spacing::PAD_12,
    bottom: spacing::PAD_8,
    left: spacing::PAD_12,
};

/// Builds a toggle row. The whole capsule dispatches `message` on press, the
/// trailing switch dispatches the same message on its own track. A `None`
/// message is the disabled state, inert across the whole row.
pub fn switch_row<'a, Message>(
    label: &'a str,
    sub: Option<&'a str>,
    on: bool,
    message: Option<Message>,
) -> Element<'a, Message>
where
    Message: Clone + 'a,
{
    let trail = switch(on, SwitchSizeVariant::Alt, message.clone());

    // Mirrors icon_flat's disabled cue: the row carries no hover/press
    // feedback when inert, so the label has to be the visual signal.
    let label_variant = if message.is_some() {
        text::Variant::Main
    } else {
        text::Variant::Alt
    };

    let sub_color = if message.is_some() {
        color::TEXT_SECONDARY
    } else {
        color::TEXT_DARK
    };

    let mut text_column = Column::new().spacing(spacing::GAP_2);
    text_column =
        text_column.push(text::label(label, label_variant).font(font::medium()));
    if let Some(sub) = sub {
        text_column = text_column.push(text::caption(sub).color(sub_color));
    }

    let content = Row::new()
        .push(text_column)
        .push(Space::new().width(Length::Fill))
        .push(trail)
        .spacing(spacing::GAP_12)
        .align_y(alignment::Vertical::Center);

    clickable(content)
        .tint(color::border())
        .radius(border::ROUNDED_FULL)
        .padding(ROW_PADDING)
        .width(Length::Fill)
        .on_press_maybe(message)
        .into()
}
