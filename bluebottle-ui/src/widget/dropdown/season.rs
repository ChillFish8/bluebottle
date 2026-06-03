//! Season selector. The reference r8 panel dropdown.
//!
//! Exposes two layers. [`panel`] is the bare panel chassis that
//! [`super::filter`] and [`super::labelled`] share for their chrome. [`season`]
//! is the data-driven builder on top. The caller hands it a slice of
//! [`SeasonInfo`] and the picked-index callback, and the widget owns the
//! `expanded` state internally.

use std::borrow::Cow;

use iced::widget::{Row, Space, column, container};
use iced::{Color, Element, Length, Padding, alignment, padding};

use super::chassis::{Dropdown, dropdown};
use crate::widget::clickable::{Clickable, clickable};
use crate::widget::ellipsis_text::ellipsis_text;
use crate::widget::text;
use crate::{color, font, icon};

const TRIGGER_RADIUS: f32 = 8.0;

const TRIGGER_PADDING: Padding = Padding {
    top: 6.0,
    right: 8.0,
    bottom: 6.0,
    left: 12.0,
};

const MENU_RADIUS: f32 = 12.0;
const MENU_WIDTH: f32 = 220.0;

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

/// The bare panel chassis shared by season, filter, and labelled.
///
/// The trigger reads as an r8 panel pill. The fill, hover veil, and resting
/// hairline match the bordered glass icon button so the pill slots into
/// toolbars beside one. While the menu is open the fill stays put and only
/// the hairline swaps to the accent colour. The menu drops below as a deep
/// violet-glass surface at r12.
pub fn panel<'a, Message>(
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
        .background(color::border())
        .tint(color::hover_veil())
        .border(color::border_strong())
        .selected_border(color::primary())
        .menu_background(color::GLASS_OPAQUE)
        .menu_border(color::border())
        .menu_radius(MENU_RADIUS)
        .menu_padding(MENU_PADDING)
        .menu_width(Length::Fixed(MENU_WIDTH))
}

/// A season menu row.
///
/// Pass `selected = true` for the active option so the row paints the accent
/// fill and the label and tick cascade through `primary()`. Unselected rows
/// stay transparent and lean on the row-hover veil for affordance.
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
        .selected_color(color::primary())
        .radius(ROW_RADIUS)
        .padding(ROW_PADDING)
        .width(Length::Fill)
}

const TRIGGER_GAP: f32 = 8.0;

const ROW_LINE_SPACING: f32 = 2.0;
const MENU_ROW_SPACING: f32 = 4.0;

const ROW_WIDTH: f32 = 260.0;
const SEASON_MENU_WIDTH: f32 = ROW_WIDTH + MENU_PADDING.left + MENU_PADDING.right;

const TICK_GLYPH_SIZE: f32 = 14.0;
pub(super) const TICK_GAP: f32 = 8.0;
const TITLE_YEAR_GAP: f32 = 6.0;

/// The check glyph used in the left column of a menu row. Renders accent
/// colour when `selected` and transparent otherwise so the tick column
/// reserves the same width on every row.
pub(super) fn tick_glyph<'a>(selected: bool) -> crate::widget::text::Text<'a> {
    let tick_color = if selected {
        color::primary()
    } else {
        Color::TRANSPARENT
    };
    icon::filled("check")
        .size(TICK_GLYPH_SIZE)
        .color(tick_color)
}

/// One season's worth of menu data. Build with [`season_info`].
#[derive(Clone)]
pub struct SeasonInfo {
    title: Cow<'static, str>,
    subtitle: Cow<'static, str>,
    year: u32,
    episode_count: u32,
}

/// Builds a [`SeasonInfo`] from a title, a subtitle, an air year, and the
/// number of episodes. The subtitle, year, and episode count compose into the
/// dim second line of the menu row.
pub fn season_info(
    title: impl Into<Cow<'static, str>>,
    subtitle: impl Into<Cow<'static, str>>,
    year: u32,
    episode_count: u32,
) -> SeasonInfo {
    SeasonInfo {
        title: title.into(),
        subtitle: subtitle.into(),
        year,
        episode_count,
    }
}

/// A self-managing season dropdown.
///
/// The widget owns its open state. The trigger label tracks `items[selected]`
/// and the menu rows render the rich two-line layout per [`SeasonInfo`]. Each
/// row presses with `on_select(i)` where `i` is the zero-based index into the
/// supplied iterator. The chassis chrome comes from [`panel`].
pub fn season<'a, Message>(
    items: impl IntoIterator<Item = SeasonInfo>,
    selected: usize,
    on_select: impl Fn(usize) -> Message + 'a,
) -> Dropdown<'a, Message>
where
    Message: Clone + 'a,
{
    let items: Vec<SeasonInfo> = items.into_iter().collect();

    let trigger_width = trigger_width(&items);
    let trigger_label: Element<'a, Message> =
        trigger_label(&items, selected, trigger_width);

    let mut menu = column![].spacing(MENU_ROW_SPACING).width(Length::Fill);

    for (index, item) in items.iter().enumerate() {
        menu = menu.push(row(
            season_row_content(item, index == selected),
            index == selected,
            on_select(index),
        ));
    }

    panel(trigger_label, menu, false).menu_width(Length::Fixed(SEASON_MENU_WIDTH))
}

/// Computes the fixed trigger row width. Rounds the widest natural row up to
/// the nearest 10 px so the trigger stays stable across selections.
fn trigger_width(items: &[SeasonInfo]) -> f32 {
    let widest = items
        .iter()
        .map(|item| {
            trigger_title_text(item).shape_width()
                + TRIGGER_GAP
                + trigger_eps_text(item).shape_width()
        })
        .fold(0.0, f32::max);

    (widest / 10.0).ceil() * 10.0
}

fn trigger_title_text<'a>(item: &SeasonInfo) -> text::Text<'a> {
    text::label(item.title.clone(), text::Variant::Main).font(font::semibold())
}

fn trigger_eps_text<'a>(item: &SeasonInfo) -> text::Text<'a> {
    text::caption(format!("· {} eps", item.episode_count)).font(font::medium())
}

/// Builds the trigger row at the precomputed fixed width. Title sits flush
/// left, episode count flush right, with a fill spacer between.
fn trigger_label<'a, Message>(
    items: &[SeasonInfo],
    selected: usize,
    width: f32,
) -> Element<'a, Message>
where
    Message: Clone + 'a,
{
    let inner = match items.get(selected) {
        Some(item) => {
            let title = trigger_title_text(item);
            let optically_aligned = container(title).padding(padding::bottom(1));

            Row::new()
                .push(optically_aligned)
                .push(Space::new().width(Length::Fill))
                .push(trigger_eps_text(item))
        },
        None => Row::new().push(Space::new().width(Length::Fill)),
    };

    inner
        .width(Length::Fixed(width))
        .align_y(alignment::Vertical::Center)
        .into()
}

/// Builds the two-line content inside a single season row. The tick column
/// reserves space even when not selected so the title columns line up across
/// rows.
fn season_row_content<'a, Message>(
    item: &SeasonInfo,
    selected: bool,
) -> Element<'a, Message>
where
    Message: Clone + 'a,
{
    let tick = tick_glyph(selected);

    let season_title = text::card_title(item.title.clone())
        .font(font::semibold())
        .color(color::TEXT_PRIMARY);
    let year = text::micro_label(item.year.to_string())
        .font(font::medium())
        .color(color::TEXT_SECONDARY);

    let title_line = Row::new()
        .push(season_title)
        .push(year)
        .spacing(TITLE_YEAR_GAP)
        .align_y(alignment::Vertical::Center);

    let subtitle = ellipsis_text(
        text::micro_label(item.subtitle.clone())
            .font(font::regular_italic())
            .color(color::TEXT_SECONDARY),
    )
    .width(Length::Fill);

    let main = column![title_line, subtitle]
        .spacing(ROW_LINE_SPACING)
        .width(Length::Fill);

    let eps = text::micro_label(format!("{} eps", item.episode_count))
        .font(font::medium())
        .color(color::TEXT_SECONDARY);

    Row::new()
        .push(tick)
        .push(main)
        .push(eps)
        .spacing(TICK_GAP)
        .align_y(alignment::Vertical::Center)
        .into()
}
