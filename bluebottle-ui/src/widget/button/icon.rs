use iced::Element;

use super::chassis::icon_circle;
use crate::color;
use crate::widget::clickable::clickable;

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
    clickable(icon_circle(icon_name, diameter, icon_size))
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

    clickable(icon_circle(icon_name, 38.0, 16.0))
        .tint(color::border_strong())
        .resting_color(resting)
        .selected(on)
        .selected_color(color::primary())
        .on_press_maybe(message)
        .into()
}

/// Icon · Carousel Nav
///
/// The smallest icon circle, for paging chevrons in section headers. A 4% white
/// fill behind a 6% hairline, both lifting to 9% / 10% on hover. An interactive
/// chevron keeps a white glyph; passing `None` for the message dims the glyph
/// to text-secondary and holds the same footprint, signalling the direction of
/// travel is unavailable.
pub fn icon_carousel<'a, Message>(
    icon_name: &'a str,
    message: Option<Message>,
) -> Element<'a, Message>
where
    Message: Clone + 'a,
{
    let glyph = if message.is_some() {
        color::TEXT_PRIMARY
    } else {
        color::TEXT_SECONDARY
    };

    clickable(icon_circle(icon_name, 26.0, 14.0))
        .background(color::hover_veil())
        .tint(color::with_alpha(color::WHITE, color::srgb_alpha(0.05)))
        .border(color::border())
        .hover_border(color::hover_veil())
        .resting_color(glyph)
        .on_press_maybe(message)
        .into()
}

/// Icon · Overlay Pill
///
/// For controls floating directly over artwork or video. Carries an 8% white
/// fill behind a 10% hairline so it stays legible over any frame beneath. Hover
/// brightens the fill to 16%. The 10px backdrop blur called for by the spec is
/// the responsibility of the surface this pill is placed on. iced does not
/// composite a backdrop pass and adding one here would require a custom shader.
pub fn icon_overlay<'a, Message>(
    icon_name: &'a str,
    message: Message,
) -> Element<'a, Message>
where
    Message: Clone + 'a,
{
    clickable(icon_circle(icon_name, 32.0, 16.0))
        .background(color::with_alpha(color::WHITE, color::srgb_alpha(0.08)))
        .tint(color::with_alpha(color::WHITE, color::srgb_alpha(0.08)))
        .border(color::border_strong())
        .on_press(message)
        .into()
}
