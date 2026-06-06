//! Compact meta chips for badges, categories, and tags.
//!
//! Three rounded-rectangle forms that share the same footprint. An
//! [informational] wraps a micro-label run in a hairline border with no fill.
//! A [category] carries the active toggle pill's accent recipe at caption
//! size. A [tag] wears the bordered-glass icon's neutral fill at caption size
//! without the ring. Each becomes interactive the moment an `on_press` is
//! supplied.

use iced::widget::text::IntoFragment;
use iced::{Element, padding};

use crate::color;
use crate::widget::clickable::clickable;
use crate::widget::text;

/// Corner radius of every meta chip. Rounded rectangle, not pill.
const META_RADIUS: f32 = 6.0;

/// Shared chip padding. 4 vertical, 8 horizontal.
fn meta_padding() -> padding::Padding {
    padding::Padding::default().vertical(4).horizontal(8)
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
        .radius(META_RADIUS)
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
        .radius(META_RADIUS)
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
        .radius(META_RADIUS)
        .background(color::border())
        .tint(color::hover_veil())
        .on_press_maybe(on_press)
        .into()
}
