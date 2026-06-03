//! Labelled-prefix dropdown. Inherits [`super::season`]'s panel chrome.
//!
//! Pairs a caller-supplied prefix string with the chosen value in the trigger.
//! Reads as `prefix value` followed by the chevron. Use for sort, quality,
//! view, or any single-axis picker where the value is the emphasis and a
//! short prefix clarifies the axis.
//!
//! The widget owns its own open state. The caller passes the choices as an
//! iterator of strings and a current index. Selection emits the picked index
//! through `on_select`. The trigger row width snaps to the longest natural
//! label-plus-value pairing, rounded up to the nearest 10 px, so picking
//! between values does not jitter the chevron.

use std::borrow::Cow;

use iced::widget::{Row, column};
use iced::{Length, alignment};

use super::chassis::Dropdown;
use super::season;
use crate::widget::text;
use crate::{color, font};

const PREFIX_GAP: f32 = 4.0;
const MENU_ROW_SPACING: f32 = 4.0;

/// A self-managing labelled-prefix dropdown.
///
/// The trigger reads as `label value` to the left of the chevron. `items`
/// supplies the menu choices in order. The selected index drives both the
/// trigger value and the menu's checked row. Each row presses with
/// `on_select(i)`. The chrome inherits [`super::season::panel`].
pub fn labelled<'a, Message>(
    label: impl Into<Cow<'static, str>>,
    items: impl IntoIterator<Item = impl Into<Cow<'static, str>>>,
    selected: usize,
    on_select: impl Fn(usize) -> Message + 'a,
) -> Dropdown<'a, Message>
where
    Message: Clone + 'a,
{
    let label: Cow<'static, str> = label.into();
    let items: Vec<Cow<'static, str>> =
        items.into_iter().map(|item| item.into()).collect();

    let trigger_width = trigger_width(&label, &items);

    let value: Cow<'static, str> =
        items.get(selected).cloned().unwrap_or(Cow::Borrowed(""));

    let trigger = Row::new()
        .push(prefix_text(label))
        .push(trigger_value_text(value))
        .spacing(PREFIX_GAP)
        .width(Length::Fixed(trigger_width))
        .align_y(alignment::Vertical::Center);

    let mut menu = column![].spacing(MENU_ROW_SPACING).width(Length::Fill);

    for (index, item) in items.iter().enumerate() {
        let content = Row::new()
            .push(season::tick_glyph(index == selected))
            .push(value_text(item.clone()))
            .spacing(season::TICK_GAP)
            .align_y(alignment::Vertical::Center);

        menu = menu.push(season::row(content, index == selected, on_select(index)));
    }

    season::panel(trigger, menu, false)
}

/// Computes the fixed trigger row width. Rounds the widest natural
/// label-plus-value pairing up to the nearest 10 px so the trigger stays
/// stable across selections.
fn trigger_width(label: &str, items: &[Cow<'static, str>]) -> f32 {
    let label_width = prefix_text(Cow::Owned(label.to_owned())).shape_width();
    let max_value_width = items
        .iter()
        .map(|item| trigger_value_text(item.clone()).shape_width())
        .fold(0.0_f32, f32::max);

    let widest = label_width + PREFIX_GAP + max_value_width;
    (widest / 10.0).ceil() * 10.0
}

fn prefix_text<'a>(content: Cow<'static, str>) -> text::Text<'a> {
    text::label(content, text::Variant::Alt).font(font::medium())
}

fn trigger_value_text<'a>(content: Cow<'static, str>) -> text::Text<'a> {
    text::label(content, text::Variant::Main).font(font::semibold())
}

fn value_text<'a>(content: Cow<'static, str>) -> text::Text<'a> {
    text::card_title(content)
        .font(font::semibold())
        .color(color::TEXT_PRIMARY)
}
