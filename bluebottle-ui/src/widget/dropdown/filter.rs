//! Multi-select filter dropdown built on season's panel chrome. The trigger
//! reads `label · N` and the menu opens onto a pinned header with a Select
//! all or Clear link above the row list.
//!
//! Each row is a bordered glass checkbox and a label. The row chrome stays
//! transparent across states so a fully-on menu does not flood with accent.

use std::borrow::Cow;

use iced::widget::{Row, Space, column};
use iced::{Element, Length, alignment, padding};

use super::chassis::Dropdown;
use super::{internal, season};
use crate::widget::button::{CheckboxSizeVariant, checkbox};
use crate::widget::clickable::Clickable;
use crate::widget::link::link;
use crate::widget::scrollable::scrollable;
use crate::widget::text;
use crate::{color, font};

// Matches season's `"· N eps"` prefix so the count text reads the same on
// both triggers. The label-to-count gap is provided by the fill spacer in
// the trigger row, not by leading whitespace here.
const SUFFIX_GAP_HINT: &str = "\u{00b7} ";

// Minimum visible gap between the label and the count inside the trigger
// row. Folded into `trigger_width` so the rounded width always leaves at
// least this much slack for the fill spacer to consume. Without it, a label
// whose natural width lands close to a 10 px boundary can crash into the
// count.
const TRIGGER_MIN_GAP: f32 = 12.0;

const MENU_INNER_SPACING: f32 = 4.0;

const MAX_ROWS: usize = 6;
// Each row is the 20 px checkbox plus ROW_PADDING (6 + 6). The 4 px slack
// covers descender depth on the row title across font fallbacks.
const ROW_FULL_HEIGHT: f32 = 40.0;
const ROWS_CAP: f32 = (MAX_ROWS as f32) * ROW_FULL_HEIGHT
    + ((MAX_ROWS - 1) as f32) * internal::MENU_ROW_SPACING;

/// A self-managing filter dropdown. Rows toggle through `on_toggle(i)` and
/// the header link fires `on_bulk(true)` for Select all or `on_bulk(false)`
/// for Clear.
pub fn filter<'a, Message>(
    label: impl Into<Cow<'static, str>>,
    items: impl IntoIterator<Item = impl Into<Cow<'static, str>>>,
    checked: impl IntoIterator<Item = bool>,
    on_toggle: impl Fn(usize) -> Message + 'a,
    on_bulk: impl Fn(bool) -> Message + 'a,
) -> Dropdown<'a, Message>
where
    Message: Clone + 'a,
{
    let label: Cow<'static, str> = label.into();
    let items: Vec<Cow<'static, str>> =
        items.into_iter().map(|item| item.into()).collect();
    let checked: Vec<bool> = checked.into_iter().collect();

    let active = checked.iter().filter(|c| **c).count();

    let trigger_width = trigger_width(&label, items.len());
    let header = header_row(label.as_ref(), active, &on_bulk);
    let trigger = trigger_label(label, active, trigger_width);

    let mut rows = column![]
        .spacing(internal::MENU_ROW_SPACING)
        .width(Length::Fill);
    for (index, item) in items.iter().enumerate() {
        let is_checked = checked.get(index).copied().unwrap_or(false);
        rows = rows.push(row(
            menu_row_content(item.clone(), is_checked),
            is_checked,
            on_toggle(index),
        ));
    }

    let scroll = scrollable(rows)
        .max_height(ROWS_CAP)
        .fade_edges(color::GLASS_OPAQUE);

    let menu = column![header, scroll]
        .spacing(MENU_INNER_SPACING)
        .width(Length::Fill);

    season::panel(trigger, menu, false)
}

/// A filter menu row. The checkbox carries the state and the row chrome
/// stays transparent in every state.
pub fn row<'a, Message>(
    content: impl Into<Element<'a, Message>>,
    checked: bool,
    on_press: Message,
) -> Clickable<'a, Message>
where
    Message: Clone + 'a,
{
    internal::row(content, checked, on_press)
        .resting_color(color::TEXT_PRIMARY)
        .selected_color(color::primary())
}

/// Fixed trigger row width. Shapes the label and the widest count fragment
/// under a single font lock, then rounds up to the nearest 10 px.
fn trigger_width(label: &str, item_count: usize) -> f32 {
    let label_text = base_text(label);
    let count_text = count_text(item_count);
    let widths = text::shape_widths([&label_text, &count_text]);

    internal::round_up_10_min(
        widths[0] + TRIGGER_MIN_GAP + widths[1],
        internal::TRIGGER_MIN_WIDTH,
    )
}

fn trigger_label<'a, Message>(
    label: Cow<'static, str>,
    active: usize,
    width: f32,
) -> Element<'a, Message>
where
    Message: Clone + 'a,
{
    // Same layout as `season::trigger_label`: title flush left, fill spacer,
    // count flush right. Width is fixed so the chevron stays put across
    // selection counts.
    let mut row = Row::new()
        .push(base_text(label))
        .push(Space::new().width(Length::Fill))
        .align_y(alignment::Vertical::Center);

    if active > 0 {
        row = row.push(count_text(active));
    }

    row.padding(padding::bottom(1))
        .width(Length::Fixed(width))
        .into()
}

fn base_text<'a>(content: impl iced::widget::text::IntoFragment<'a>) -> text::Text<'a> {
    internal::trigger_main_text(content)
}

fn count_text<'a>(active: usize) -> text::Text<'a> {
    internal::count_caption(format!("{SUFFIX_GAP_HINT}{active}"))
}

fn header_row<'a, Message>(
    label: &str,
    active: usize,
    on_bulk: &(impl Fn(bool) -> Message + 'a),
) -> Element<'a, Message>
where
    Message: Clone + 'a,
{
    let with_seperator = format!("{} •", label.to_uppercase());
    let label =
        text::eyebrow(with_seperator, text::Variant::Main).color(color::TEXT_SECONDARY);
    let select_count = text::eyebrow(format!("{active} SELECTED"), text::Variant::Main)
        .color(color::TEXT_MUTED);

    let summary = iced::widget::row![label, select_count].spacing(4);

    let (action_label, action_value) = if active > 0 {
        ("Clear", false)
    } else {
        ("Select all", true)
    };

    let action = link(
        text::label(action_label, text::Variant::Main).font(font::semibold()),
        on_bulk(action_value),
    );

    internal::header_row(summary, action)
}

fn menu_row_content<'a, Message>(
    label: Cow<'static, str>,
    checked: bool,
) -> Element<'a, Message>
where
    Message: Clone + 'a,
{
    let box_ = checkbox::<Message>(checked, CheckboxSizeVariant::Alt, None);

    let title = text::card_title(label)
        .font(font::semibold())
        .inherit_color();

    Row::new()
        .push(box_)
        .push(title)
        .spacing(internal::TICK_GAP)
        .align_y(alignment::Vertical::Center)
        .into()
}
