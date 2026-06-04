//! Season selector. The reference r8 panel dropdown that filter and labelled
//! inherit their chrome from.

use std::borrow::Cow;

use iced::widget::{Row, Space, column, container};
use iced::{Element, Length, Padding, alignment, padding};

use super::chassis::{Dropdown, dropdown};
use super::internal;
use crate::widget::clickable::Clickable;
use crate::widget::ellipsis_text::ellipsis_text;
use crate::widget::scrollable::scrollable;
use crate::widget::text;
use crate::{color, font};

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

/// The bare panel chassis shared by season, filter, and labelled. An r8 pill
/// trigger with bordered glass chrome over an r12 violet-glass menu.
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

/// A season menu row. The selected row paints the accent fill while
/// unselected rows stay transparent and lean on the hover veil.
pub fn row<'a, Message>(
    content: impl Into<Element<'a, Message>>,
    selected: bool,
    on_press: Message,
) -> Clickable<'a, Message>
where
    Message: Clone + 'a,
{
    internal::row(content, selected, on_press)
        .resting_color(color::TEXT_PRIMARY)
        .selected_color(color::primary())
        .selected_background(color::accent_row_selected())
}

const TRIGGER_GAP: f32 = 8.0;

const ROW_LINE_SPACING: f32 = 2.0;

const ROW_WIDTH: f32 = 260.0;
const SEASON_MENU_WIDTH: f32 = ROW_WIDTH + MENU_PADDING.left + MENU_PADDING.right;

const TITLE_YEAR_GAP: f32 = 6.0;

const MAX_ROWS: usize = 4;
const ROW_FULL_HEIGHT: f32 = 56.0;
const ROWS_CAP: f32 = (MAX_ROWS as f32) * ROW_FULL_HEIGHT
    + ((MAX_ROWS - 1) as f32) * internal::MENU_ROW_SPACING;

/// One season's worth of menu data. Build with [`season_info`].
#[derive(Clone)]
pub struct SeasonInfo {
    title: Cow<'static, str>,
    subtitle: Cow<'static, str>,
    year: u32,
    episode_count: u32,
}

/// Builds a [`SeasonInfo`]. The subtitle, year, and episode count compose
/// into the dim second line of the menu row.
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

/// A self-managing season dropdown. The trigger label tracks
/// `items[selected]` and each menu row presses with `on_select(i)`.
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

    let mut rows = column![]
        .spacing(internal::MENU_ROW_SPACING)
        .width(Length::Fill);

    for (index, item) in items.iter().enumerate() {
        rows = rows.push(row(
            season_row_content(item, index == selected),
            index == selected,
            on_select(index),
        ));
    }

    let menu = scrollable(rows)
        .max_height(ROWS_CAP)
        .fade_edges(color::GLASS_OPAQUE);

    panel(trigger_label, menu, false).menu_width(Length::Fixed(SEASON_MENU_WIDTH))
}

/// Fixed trigger row width. Shapes title and eps for every item in one font
/// lock, sums each pair, and rounds the widest sum up to the nearest 10 px.
fn trigger_width(items: &[SeasonInfo]) -> f32 {
    let mut runs: Vec<text::Text<'_>> = Vec::with_capacity(items.len() * 2);
    for item in items {
        runs.push(trigger_title_text(item.title.as_ref()));
        runs.push(trigger_eps_text(item));
    }

    let widths = text::shape_widths(runs.iter());
    let widest = widths
        .chunks_exact(2)
        .map(|pair| pair[0] + TRIGGER_GAP + pair[1])
        .fold(0.0_f32, f32::max);

    internal::round_up_10_min(widest, internal::TRIGGER_MIN_WIDTH)
}

fn trigger_title_text<'a>(
    content: impl iced::widget::text::IntoFragment<'a>,
) -> text::Text<'a> {
    internal::trigger_main_text(content)
}

fn trigger_eps_text<'a>(item: &SeasonInfo) -> text::Text<'a> {
    internal::count_caption(format!("· {} eps", item.episode_count))
}

/// Builds the trigger row at the precomputed fixed width. Title flush left,
/// episode count flush right.
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
            let title = trigger_title_text(item.title.clone());
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

/// Two-line content for one season row. The tick column reserves its slot
/// even when not selected so titles line up across rows.
fn season_row_content<'a, Message>(
    item: &SeasonInfo,
    selected: bool,
) -> Element<'a, Message>
where
    Message: Clone + 'a,
{
    let tick = internal::tick_glyph(selected);

    let season_title = text::card_title(item.title.clone())
        .font(font::semibold())
        .inherit_color();
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
        .spacing(internal::TICK_GAP)
        .align_y(alignment::Vertical::Center)
        .into()
}
