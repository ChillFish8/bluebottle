use iced::widget::container;
use iced::{Center, Element, Length};

use crate::widget::clickable::clickable;
use crate::{color, icon};

/// Size of an icon circle. Only the two specced sizes exist so the icons stay
/// consistent across rows. Mirrors [`text::Variant`](crate::text::Variant).
#[derive(Debug, Eq, PartialEq, Copy, Clone, Default)]
pub enum IconSizeVariant {
    #[default]
    Main,
    Alt,
}

/// Icon · Bordered Glass
///
/// The hero-row workhorse. A circular white glass fill behind a hairline with a
/// centred white glyph. Hover brightens the fill. When `on` it fills with a soft
/// accent behind a full accent ring, the saved or liked state.
pub fn icon<'a, Message>(
    icon_name: &'a str,
    size: IconSizeVariant,
    on: bool,
    message: Message,
) -> Element<'a, Message>
where
    Message: Clone + 'a,
{
    let (diameter, icon_size) = match size {
        IconSizeVariant::Main => (36.0, 14.0),
        IconSizeVariant::Alt => (38.0, 16.0),
    };

    // The white fill brightens on hover through the tint. The on state swaps to
    // the accent fill and ring while the glyph stays white.
    clickable(circle(icon_name, diameter, icon_size))
        .background(color::border())
        .tint(color::hover_veil())
        .border(color::border_strong())
        .selected(on)
        .selected_background(color::primary_glass())
        .selected_border(color::primary())
        .on_press(message)
        .into()
}

/// Icon · Flat Round
///
/// A border-free icon circle for denser rows. Transparent at rest with a white
/// glyph. Hover fills with a white glass veil. When `on` the ground stays clear
/// and the glyph tints to accent. A `None` message is the disabled state, with
/// the glyph dropped to text-dark and the same footprint so the row holds.
pub fn icon_flat<'a, Message>(
    icon_name: &'a str,
    on: bool,
    message: Option<Message>,
) -> Element<'a, Message>
where
    Message: Clone + 'a,
{
    // Disabled drops the resting glyph to text-dark. The on state still rides on
    // top through the selected_color cascade so a disabled-but-on icon reads as
    // accent-tinted rather than collapsing back to off.
    let resting = if message.is_some() {
        color::TEXT_PRIMARY
    } else {
        color::TEXT_DARK
    };

    clickable(circle(icon_name, 38.0, 16.0))
        .tint(color::border_strong())
        .resting_color(resting)
        .selected(on)
        .selected_color(color::primary())
        .on_press_maybe(message)
        .into()
}

/// Centred glyph in a fixed circle. Shared chassis for both variants.
fn circle<'a, Message>(
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
