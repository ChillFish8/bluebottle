use iced::widget::container;
use iced::{Center, Element, Length};

use crate::color;
use crate::widget::animated_tick::animated_tick;
use crate::widget::clickable::clickable;

/// Size variants of the checkbox.
#[derive(Debug, Eq, PartialEq, Copy, Clone, Default)]
pub enum CheckboxSizeVariant {
    /// 24px main size
    #[default]
    Main,
    /// 20px alternative size
    Alt,
}

/// Checkbox · Bordered Glass
///
/// A fully rounded bordered glass box housing the animated check. Off, it
/// reads as the white glass fill behind a hairline. On, it crossfades to the
/// accent fill and ring while the check strokes in. A `None` message makes
/// the box inert so an outer clickable row can own the press dispatch.
pub fn checkbox<'a, Message>(
    on: bool,
    size: CheckboxSizeVariant,
    message: Option<Message>,
) -> Element<'a, Message>
where
    Message: Clone + 'a,
{
    let (diameter, tick_size) = match size {
        CheckboxSizeVariant::Main => (24.0, 14.0),
        CheckboxSizeVariant::Alt => (20.0, 12.0),
    };

    let glyph = animated_tick(on, tick_size);

    let checkbox_container = container(glyph)
        .width(Length::Fixed(diameter))
        .height(Length::Fixed(diameter))
        .align_x(Center)
        .align_y(Center);

    clickable(checkbox_container)
        .background(color::border())
        .tint(color::hover_veil())
        .border(color::border_strong())
        .selected(on)
        .selected_background(color::primary_glass())
        .selected_border(color::primary())
        .on_press_maybe(message)
        .into()
}
