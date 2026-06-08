//! Compact meta chips for badges, categories, and tags.
//!
//! Three rounded-rectangle forms that share the same footprint. An
//! [informational] wraps a micro-label run in a hairline border with no fill.
//! A [category] carries the active toggle pill's accent recipe at caption
//! size. A [tag] wears the bordered-glass icon's neutral fill at caption size
//! without the ring. Each becomes interactive the moment an `on_press` is
//! supplied.

use iced::widget::row;
use iced::widget::text::IntoFragment;
use iced::{Center, Element, padding};

use crate::widget::clickable::clickable;
use crate::widget::text;
use crate::{border, color, font, icon, spacing};

/// Shared chip padding. 4 vertical, 8 horizontal.
fn meta_padding() -> padding::Padding {
    padding::Padding::default()
        .vertical(spacing::PAD_4)
        .horizontal(spacing::PAD_8)
}

/// A hairline-bordered chip around a micro-label run. The transparent fill keeps
/// the chip quiet against any surface. Use for inline metadata badges.
pub fn informational<'a, Message>(
    label: impl IntoFragment<'a>,
    on_press: Option<Message>,
) -> Element<'a, Message>
where
    Message: Clone + 'a,
{
    clickable(text::micro_label(label).color(color::TEXT_SECONDARY))
        .padding(meta_padding())
        .radius(border::ROUNDED_SM)
        .border(color::border_strong())
        .tint(color::hover_veil())
        .on_press_maybe(on_press)
        .into()
}

/// A chip carrying the active toggle pill's recipe. A soft accent fill behind
/// accent-tinted caption text. Use for genre and category callouts.
pub fn category<'a, Message>(
    label: impl IntoFragment<'a>,
    on_press: Option<Message>,
) -> Element<'a, Message>
where
    Message: Clone + 'a,
{
    clickable(text::caption(label).color(color::primary()))
        .padding(meta_padding())
        .radius(border::ROUNDED_SM)
        .background(color::primary_glass())
        .tint(color::hover_veil())
        .on_press_maybe(on_press)
        .into()
}

/// A neutral bordered-glass chip without the ring. A white 6% fill behind a
/// white caption. Use for free-form tags and detail-row chips.
pub fn tag<'a, Message>(
    label: impl IntoFragment<'a>,
    on_press: Option<Message>,
) -> Element<'a, Message>
where
    Message: Clone + 'a,
{
    clickable(text::caption(label).color(color::TEXT_PRIMARY))
        .padding(meta_padding())
        .radius(border::ROUNDED_SM)
        .background(color::border())
        .tint(color::hover_veil())
        .on_press_maybe(on_press)
        .into()
}

/// A full-pill frosted chip for use over imagery. White at 14% behind a
/// hairline at 16%, with a medium-weight micro label. Static. The backdrop
/// blur comes from the host [`media_image`](crate::widget::media_image),
/// so the chip itself only paints the fill, border, and label.
pub fn frosted<'a, Message>(label: impl IntoFragment<'a>) -> Element<'a, Message>
where
    Message: Clone + 'a,
{
    let fill = color::with_alpha(color::WHITE, color::srgb_alpha(0.14));
    let border = color::with_alpha(color::WHITE, color::srgb_alpha(0.16));

    clickable(
        text::micro_label(label)
            .font(font::medium())
            .letter_spacing(0.0)
            .color(color::TEXT_PRIMARY),
    )
    .padding(padding::Padding::default().vertical(5).horizontal(9))
    .background(fill)
    .border(border)
    .into()
}

/// A small accent-tint pill that labels a carousel section. An accent 10%
/// fill behind an accent 20% hairline, with a bold caption-sized accent
/// label and an optional 11px leading glyph. The flat tint sets it apart
/// from [`frosted`], which lives over imagery.
pub fn section_badge<'a, Message>(
    label: impl IntoFragment<'a>,
    icon_name: Option<&'a str>,
    on_press: Option<Message>,
) -> Element<'a, Message>
where
    Message: Clone + 'a,
{
    let accent = color::primary();
    let fill = color::with_alpha(accent, color::srgb_alpha(0.10));
    let border = color::with_alpha(accent, color::srgb_alpha(0.20));

    let mut items = row![].spacing(spacing::GAP_4).align_y(Center);
    if let Some(name) = icon_name {
        items = items.push(icon::filled(name).size(11).color(accent));
    }
    items = items.push(text::caption(label).font(font::bold()).color(accent));

    clickable(items)
        .padding(padding::Padding::default().vertical(3).horizontal(8))
        .background(fill)
        .border(border)
        .tint(color::hover_veil())
        .on_press_maybe(on_press)
        .into()
}
