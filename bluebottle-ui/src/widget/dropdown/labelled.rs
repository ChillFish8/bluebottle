//! Labelled-prefix dropdown. Inherits [`super::season`]'s panel chrome.
//!
//! Pairs a caller-supplied prefix label with the chosen value in the trigger.
//! Reads as `prefix value` followed by the chevron. Use for sort, quality,
//! view, or any single-axis picker where the value is the emphasis and a
//! short prefix clarifies the axis.
//!
//! The variant does not enforce a dim colour on the prefix. Pass a styled
//! text widget for that recipe, or any Element for a richer prefix.

use iced::widget::Row;
use iced::{Element, alignment};

use super::chassis::Dropdown;
pub use super::season::row;

const PREFIX_GAP: f32 = 4.0;

/// A labelled-prefix dropdown. The trigger reads as `label value` to the
/// left of the chevron. The chrome inherits [`super::season::panel`].
pub fn labelled<'a, Message>(
    label: impl Into<Element<'a, Message>>,
    value: impl Into<Element<'a, Message>>,
    menu: impl Into<Element<'a, Message>>,
    expanded: bool,
) -> Dropdown<'a, Message>
where
    Message: Clone + 'a,
{
    let trigger = Row::new()
        .push(label.into())
        .push(value.into())
        .spacing(PREFIX_GAP)
        .align_y(alignment::Vertical::Center);

    super::season::panel(trigger, menu, expanded)
}
