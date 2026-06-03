//! Source / library selector. A capsule chip trigger over the rich menu.
//!
//! The newest control in the family. The trigger is a fully-rounded capsule
//! reading as a status chip. The menu picks up the same deep violet-glass
//! surface as [`super::season`] but with a wider r14 radius, and the selected
//! row carries the 1 px inset accent ring that no other variant uses.

use iced::{Element, Length, Padding};

use super::chassis::{Dropdown, dropdown};
use crate::color;
use crate::widget::clickable::{Clickable, clickable};

const TRIGGER_RADIUS: f32 = 999.0;

const TRIGGER_PADDING: Padding = Padding {
    top: 6.0,
    right: 8.0,
    bottom: 6.0,
    left: 10.0,
};

const MENU_RADIUS: f32 = 14.0;
const MENU_WIDTH: f32 = 320.0;

const MENU_PADDING: Padding = Padding {
    top: 6.0,
    right: 6.0,
    bottom: 6.0,
    left: 6.0,
};

const ROW_RADIUS: f32 = 8.0;

const ROW_PADDING: Padding = Padding {
    top: 6.0,
    right: 10.0,
    bottom: 6.0,
    left: 10.0,
};

/// A source-style dropdown.
///
/// The trigger reads as a capsule chip. The fill, hover veil, and resting
/// hairline match the overlay button vocabulary so the chip slots into a
/// hero row beside one. The hairline swaps to the accent colour while the
/// menu is open. The menu surface mirrors the panel recipe at r14 so a
/// richer row layout has more breathing room.
pub fn source<'a, Message>(
    label: impl Into<Element<'a, Message>>,
    menu: impl Into<Element<'a, Message>>,
    expanded: bool,
) -> Dropdown<'a, Message>
where
    Message: Clone + 'a,
{
    dropdown(label, menu, expanded)
        .radius(TRIGGER_RADIUS)
        .padding(TRIGGER_PADDING)
        .background(color::overlay_fill())
        .tint(color::overlay_fill())
        .border(color::border_strong())
        .selected_border(color::primary())
        .menu_background(color::GLASS_OPAQUE)
        .menu_border(color::border())
        .menu_radius(MENU_RADIUS)
        .menu_padding(MENU_PADDING)
        .menu_width(Length::Fixed(MENU_WIDTH))
}

/// A source menu row.
///
/// The selected row picks up both the accent fill and the 1 px inset accent
/// ring. The ring is the spec's unique-to-source affordance, calling out the
/// active library against the richer row content.
pub fn row<'a, Message>(
    content: impl Into<Element<'a, Message>>,
    selected: bool,
    on_press: Message,
) -> Clickable<'a, Message>
where
    Message: Clone + 'a,
{
    clickable(content)
        .on_press(on_press)
        .selected(selected)
        .tint(color::overlay_fill())
        .selected_background(color::accent_row_selected())
        .selected_border(color::primary())
        .radius(ROW_RADIUS)
        .padding(ROW_PADDING)
        .width(Length::Fill)
}
