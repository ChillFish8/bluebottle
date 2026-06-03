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

use iced::widget::{Row, Space, column, container};
use iced::{Length, alignment, padding};

use super::chassis::Dropdown;
use super::season;
use crate::widget::text;
use crate::{color, font, icon};

const PREFIX_GAP: f32 = 8.0;
const MENU_ROW_SPACING: f32 = 4.0;

const TRIGGER_ICON_SIZE: f32 = 13.0;
const ICON_LABEL_GAP: f32 = 4.0;

/// A self-managing labelled-prefix dropdown.
///
/// The trigger reads as `[icon] label value` to the left of the chevron.
/// `icon` is an optional Material Icon name shown flush-left in the
/// trigger through the design system's icon widget. `items` supplies the
/// menu choices in order. `counts` runs parallel to `items` and renders
/// flush-right inside each menu row; an empty count string skips the
/// right column. The selected index drives both the trigger value and the
/// menu's checked row. Each row presses with `on_select(i)`. The chrome
/// inherits [`super::season::panel`].
pub fn labelled<'a, Message>(
    label: impl Into<Cow<'static, str>>,
    icon: Option<&'static str>,
    items: impl IntoIterator<Item = impl Into<Cow<'static, str>>>,
    selected: usize,
    counts: impl IntoIterator<Item = impl Into<Cow<'static, str>>>,
    on_select: impl Fn(usize) -> Message + 'a,
) -> Dropdown<'a, Message>
where
    Message: Clone + 'a,
{
    let label: Cow<'static, str> = label.into();
    let items: Vec<Cow<'static, str>> =
        items.into_iter().map(|item| item.into()).collect();
    let counts: Vec<Cow<'static, str>> = counts.into_iter().map(|c| c.into()).collect();

    let trigger_width = trigger_width(&label, &items, icon);

    let value: Cow<'static, str> =
        items.get(selected).cloned().unwrap_or(Cow::Borrowed(""));

    // Cluster the icon next to the label at a tighter gap than the
    // label-to-value separation so the icon decorates the axis label
    // rather than reading as a separate column.
    let mut prefix_cluster = Row::new();
    if let Some(name) = icon {
        let icon = icon::filled(name)
            .size(TRIGGER_ICON_SIZE)
            .color(color::TEXT_SECONDARY);
        let optically_aligned = container(icon).padding(padding::top(2));
        prefix_cluster = prefix_cluster.push(optically_aligned);
    }
    let prefix_cluster = prefix_cluster
        .push(prefix_text(label))
        .spacing(ICON_LABEL_GAP)
        .align_y(alignment::Vertical::Center);

    let trigger = Row::new()
        .push(prefix_cluster)
        .push(trigger_value_text(value))
        .padding(padding::bottom(1))
        .spacing(PREFIX_GAP)
        .width(Length::Fixed(trigger_width))
        .align_y(alignment::Vertical::Center);

    let mut menu = column![].spacing(MENU_ROW_SPACING).width(Length::Fill);

    for (index, item) in items.iter().enumerate() {
        let count = counts.get(index).cloned().unwrap_or(Cow::Borrowed(""));
        let content = menu_row_content(item.clone(), index == selected, count);
        menu = menu.push(season::row(content, index == selected, on_select(index)));
    }

    season::panel(trigger, menu, false)
}

fn menu_row_content<'a, Message>(
    value: Cow<'static, str>,
    selected: bool,
    count: Cow<'static, str>,
) -> iced::Element<'a, Message>
where
    Message: Clone + 'a,
{
    let mut row = Row::new()
        .push(season::tick_glyph(selected))
        .push(value_text(value))
        .spacing(season::TICK_GAP)
        .align_y(alignment::Vertical::Center);

    if !count.is_empty() {
        let count_text = text::micro_label(count)
            .font(font::medium())
            .color(color::TEXT_SECONDARY);
        row = row.push(Space::new().width(Length::Fill)).push(count_text);
    }

    row.width(Length::Fill).into()
}

/// Computes the fixed trigger row width. Rounds the widest natural
/// label-plus-value pairing up to the nearest 10 px so the trigger stays
/// stable across selections. The icon's own width and gap are added when
/// one is present so the column accommodates it.
fn trigger_width(
    label: &str,
    items: &[Cow<'static, str>],
    icon: Option<&'static str>,
) -> f32 {
    let label_width = prefix_text(Cow::Owned(label.to_owned())).shape_width();
    let max_value_width = items
        .iter()
        .map(|item| trigger_value_text(item.clone()).shape_width())
        .fold(0.0_f32, f32::max);

    let icon_width = if icon.is_some() {
        TRIGGER_ICON_SIZE + ICON_LABEL_GAP
    } else {
        0.0
    };

    let widest = icon_width + label_width + PREFIX_GAP + max_value_width;
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
        .inherit_color()
}
