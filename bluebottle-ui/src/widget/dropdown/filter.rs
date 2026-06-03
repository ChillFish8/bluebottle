//! Multi-select filter. Inherits [`super::season`]'s panel chrome.
//!
//! The trigger reads `label · N` where N is the active-pick count, omitting
//! the count fragment when nothing is selected. The trigger row width snaps
//! to the widest possible rendering so the chevron stays put as picks come
//! and go. Rows differ from season. The checkbox glyph alone signals state,
//! so the row stays transparent regardless of checked and the hover veil is
//! the only fill that ever paints. This keeps multiple-on filter menus from
//! flooding with accent colour.
//!
//! The widget owns its own open state. Toggling a row does not close the
//! menu, so the user can flip several entries in one session.

use std::borrow::Cow;

use iced::widget::{Row, column};
use iced::{Element, Length, Padding, alignment, padding};

use super::chassis::Dropdown;
use super::season;
use crate::widget::clickable::{Clickable, clickable};
use crate::widget::text;
use crate::{color, font, icon};

const ROW_RADIUS: f32 = 8.0;

const ROW_PADDING: Padding = Padding {
    top: 6.0,
    right: 10.0,
    bottom: 6.0,
    left: 10.0,
};

const MENU_ROW_SPACING: f32 = 4.0;

const CHECKBOX_SIZE: f32 = 16.0;
const SUFFIX_GAP_HINT: &str = " \u{00b7} ";

/// A self-managing filter dropdown.
///
/// `items` supplies the row labels in order. `checked` runs parallel and
/// gives each row's current state. Pressing a row fires `on_toggle(i)`. The
/// trigger reads `label` plus a middle-dot count of the active picks. Width
/// snaps to the widest natural rendering so the chevron stays stable across
/// selection counts.
pub fn filter<'a, Message>(
    label: impl Into<Cow<'static, str>>,
    items: impl IntoIterator<Item = impl Into<Cow<'static, str>>>,
    checked: impl IntoIterator<Item = bool>,
    on_toggle: impl Fn(usize) -> Message + 'a,
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
    let trigger = trigger_label(label, active, trigger_width);

    let mut menu = column![].spacing(MENU_ROW_SPACING).width(Length::Fill);
    for (index, item) in items.iter().enumerate() {
        let is_checked = checked.get(index).copied().unwrap_or(false);
        menu = menu.push(row(
            menu_row_content(item.clone(), is_checked),
            on_toggle(index),
        ));
    }

    season::panel(trigger, menu, false)
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

/// Computes the fixed trigger row width. Rounds the widest natural rendering
/// up to the nearest 10 px so the trigger stays stable across selection
/// counts.
fn trigger_width(label: &str, item_count: usize) -> f32 {
    let label_width = base_text(Cow::Owned(label.to_owned())).shape_width();
    let suffix_width = count_text(item_count).shape_width();
    let widest = label_width + suffix_width;
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
    let mut row = Row::new()
        .push(base_text(label))
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

fn menu_row_content<'a, Message>(
    label: Cow<'static, str>,
    checked: bool,
) -> Element<'a, Message>
where
    Message: Clone + 'a,
{
    let glyph_name = if checked {
        "check_box"
    } else {
        "check_box_outline_blank"
    };
    let glyph = icon::filled(glyph_name).size(CHECKBOX_SIZE);

    let title = text::card_title(label)
        .font(font::semibold())
        .color(color::TEXT_PRIMARY);

    Row::new()
        .push(glyph)
        .push(title)
        .spacing(season::TICK_GAP)
        .align_y(alignment::Vertical::Center)
        .into()
}
