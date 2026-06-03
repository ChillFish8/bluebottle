//! Multi-select filter. Inherits [`super::season`]'s panel chrome.
//!
//! The trigger and menu match the season recipe. Rows differ. The checkbox
//! glyph alone signals state, so the row stays transparent regardless of
//! `checked` and the hover veil is the only fill that ever paints. This keeps
//! multiple-on filter menus from flooding with accent colour.

use iced::{Element, Length, Padding};

use super::chassis::Dropdown;
use crate::color;
use crate::widget::clickable::{Clickable, clickable};

const ROW_RADIUS: f32 = 8.0;

const ROW_PADDING: Padding = Padding {
    top: 6.0,
    right: 10.0,
    bottom: 6.0,
    left: 10.0,
};

/// A filter-style dropdown. Same chrome as [`super::season::panel`].
///
/// The active-pick count ("· N") in the trigger label is composed by the
/// caller. The variant takes the label as-is.
pub fn filter<'a, Message>(
    label: impl Into<Element<'a, Message>>,
    menu: impl Into<Element<'a, Message>>,
    expanded: bool,
) -> Dropdown<'a, Message>
where
    Message: Clone + 'a,
{
    super::season::panel(label, menu, expanded)
}

/// A filter menu row.
///
/// `content` carries the checkbox glyph alongside the row label. The row
/// never paints a selected fill or ring. State is communicated through the
/// checkbox the caller supplies inside `content`.
pub fn row<'a, Message>(
    content: impl Into<Element<'a, Message>>,
    on_press: Message,
) -> Clickable<'a, Message>
where
    Message: Clone + 'a,
{
    clickable(content)
        .on_press(on_press)
        .tint(color::overlay_fill())
        .radius(ROW_RADIUS)
        .padding(ROW_PADDING)
        .width(Length::Fill)
}
