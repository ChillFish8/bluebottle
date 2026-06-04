//! Multi-select filter. Inherits [`super::season`]'s panel chrome.
//!
//! The trigger reads `label · N` where N is the active-pick count, omitting
//! the count fragment when nothing is selected. The trigger row width snaps
//! to the widest possible rendering so the chevron stays put as picks come
//! and go. The menu opens onto a pinned header carrying `label — N selected`
//! on the left and a Select all or Clear link on the right, followed by the
//! row list. Rows are capped at six before scrolling, with fades at each
//! edge so off-band rows dissolve into the menu surface.
//!
//! Row style differs from season. The bordered glass checkbox alone signals
//! state, so the row stays transparent regardless of checked and the hover
//! veil is the only fill that ever paints. This keeps multiple-on filter
//! menus from flooding with accent colour.
//!
//! The widget owns its own open state. Toggling a row does not close the
//! menu, so the user can flip several entries in one session.

use std::borrow::Cow;

use iced::widget::{Row, Space, column};
use iced::{Element, Length, Padding, alignment, padding};

use super::chassis::Dropdown;
use super::season;
use crate::widget::button::{CheckboxSizeVariant, checkbox};
use crate::widget::clickable::{Clickable, clickable};
use crate::widget::link::link;
use crate::widget::scrollable::scrollable;
use crate::widget::text;
use crate::{color, font};

const ROW_RADIUS: f32 = 8.0;

const ROW_PADDING: Padding = Padding {
    top: 6.0,
    right: 10.0,
    bottom: 6.0,
    left: 10.0,
};

const MENU_ROW_SPACING: f32 = 4.0;

// Matches season's `"· N eps"` prefix so the count text reads the same on
// both triggers. The label-to-count gap is provided by the fill spacer in
// the trigger row, not by leading whitespace here.
const SUFFIX_GAP_HINT: &str = "\u{00b7} ";

// Minimum visible gap between the label and the count inside the trigger
// row. Folded into `trigger_width` so the rounded width always leaves at
// least this much slack for the fill spacer to consume — without it, a
// label whose natural width lands close to a 10 px boundary can crash
// into the count when the rounding leaves almost no slack.
const TRIGGER_MIN_GAP: f32 = 12.0;

const HEADER_PADDING: Padding = Padding {
    top: 4.0,
    right: 10.0,
    bottom: 8.0,
    left: 10.0,
};

const MENU_INNER_SPACING: f32 = 4.0;

const MAX_ROWS: usize = 6;
const ROW_FULL_HEIGHT: f32 = 36.0;
const ROWS_CAP: f32 =
    (MAX_ROWS as f32) * ROW_FULL_HEIGHT + ((MAX_ROWS - 1) as f32) * MENU_ROW_SPACING;

/// A self-managing filter dropdown.
///
/// `items` supplies the row labels in order. `checked` runs parallel and
/// gives each row's current state. Pressing a row fires `on_toggle(i)`. The
/// header carries a Select all or Clear link that fires `on_bulk(true)` or
/// `on_bulk(false)` respectively. The trigger reads `label` plus a middle
/// dot count of the active picks. Width snaps to the widest natural
/// rendering so the chevron stays stable across selection counts.
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

    let mut rows = column![].spacing(MENU_ROW_SPACING).width(Length::Fill);
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

/// A filter menu row.
///
/// `content` carries the checkbox glyph alongside the row label. The row
/// never paints a selected fill or ring. The `checked` flag drives the text
/// colour ease between resting and accent on the design system's standard
/// 100 ms hover budget. Background and ring stay transparent regardless of
/// state so multiple-on filter menus do not flood with accent colour.
pub fn row<'a, Message>(
    content: impl Into<Element<'a, Message>>,
    checked: bool,
    on_press: Message,
) -> Clickable<'a, Message>
where
    Message: Clone + 'a,
{
    clickable(content)
        .on_press(on_press)
        .selected(checked)
        .tint(color::overlay_fill())
        .resting_color(color::TEXT_PRIMARY)
        .selected_color(color::primary())
        .radius(ROW_RADIUS)
        .padding(ROW_PADDING)
        .width(Length::Fill)
}

/// Computes the fixed trigger row width. Rounds the widest natural rendering
/// up to the nearest 10 px so the trigger stays stable across selection
/// counts.
fn trigger_width(label: &str, item_count: usize) -> f32 {
    let label_width = base_text(Cow::Owned(label.to_owned())).shape_width();
    let suffix_width = count_text(item_count).shape_width();
    let widest = label_width + TRIGGER_MIN_GAP + suffix_width;
    (widest / 10.0).ceil() * 10.0
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

fn base_text<'a>(content: Cow<'static, str>) -> text::Text<'a> {
    text::label(content, text::Variant::Main).font(font::semibold())
}

fn count_text<'a>(active: usize) -> text::Text<'a> {
    text::caption(format!("{SUFFIX_GAP_HINT}{active}")).font(font::medium())
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

    Row::new()
        .push(summary)
        .push(Space::new().width(Length::Fill))
        .push(action)
        .padding(HEADER_PADDING)
        .align_y(alignment::Vertical::Center)
        .width(Length::Fill)
        .into()
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
        .spacing(season::TICK_GAP)
        .align_y(alignment::Vertical::Center)
        .into()
}
