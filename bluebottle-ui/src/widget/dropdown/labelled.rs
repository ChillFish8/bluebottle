//! Labelled-prefix dropdown built on season's panel chrome. Pairs a caller
//! prefix with the chosen value in the trigger, for any single-axis picker
//! where the value is the emphasis.

use std::borrow::Cow;

use iced::widget::{Row, Space, column, container};
use iced::{Length, alignment, padding};

use super::chassis::Dropdown;
use super::{internal, season};
use crate::widget::text;
use crate::{color, font, icon};

const PREFIX_GAP: f32 = 8.0;

const TRIGGER_ICON_SIZE: f32 = 13.0;
const ICON_LABEL_GAP: f32 = 4.0;

/// One menu choice. Carries the display name and an optional count rendered
/// flush right inside the menu row.
#[derive(Clone)]
pub struct ItemRow {
    name: Cow<'static, str>,
    count: Option<Cow<'static, str>>,
}

impl ItemRow {
    /// A new row with the given display name and no count.
    pub fn new(name: impl Into<Cow<'static, str>>) -> Self {
        Self {
            name: name.into(),
            count: None,
        }
    }

    /// Attaches a count string rendered flush-right inside the menu row.
    pub fn count(mut self, count: impl Into<Cow<'static, str>>) -> Self {
        self.count = Some(count.into());
        self
    }

    /// Attaches an optional count. `None` clears any previously-attached
    /// count.
    pub fn opt_count(mut self, count: Option<impl Into<Cow<'static, str>>>) -> Self {
        self.count = count.map(Into::into);
        self
    }
}

/// Free-function form of [`ItemRow::new`] for call-site brevity.
pub fn item_row(name: impl Into<Cow<'static, str>>) -> ItemRow {
    ItemRow::new(name)
}

/// A self-managing labelled-prefix dropdown. The trigger reads as
/// `[icon] label value` and each row presses with `on_select(i)`.
///
/// # Panics
///
/// `icon` is forwarded to [`crate::icon::filled`] without a fallback. An
/// unknown Material Icon name panics on the first draw.
pub fn labelled<'a, Message>(
    label: impl Into<Cow<'static, str>>,
    icon: Option<&'static str>,
    items: impl IntoIterator<Item = ItemRow>,
    selected: usize,
    on_select: impl Fn(usize) -> Message + 'a,
) -> Dropdown<'a, Message>
where
    Message: Clone + 'a,
{
    let label: Cow<'static, str> = label.into();
    let items: Vec<ItemRow> = items.into_iter().collect();

    let trigger_width = trigger_width(&label, &items, icon);

    let value: Cow<'static, str> = items
        .get(selected)
        .map(|item| item.name.clone())
        .unwrap_or(Cow::Borrowed(""));

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

    let mut menu = column![]
        .spacing(internal::MENU_ROW_SPACING)
        .width(Length::Fill);

    for (index, item) in items.iter().enumerate() {
        let content =
            menu_row_content(item.name.clone(), index == selected, item.count.clone());
        menu = menu.push(season::row(content, index == selected, on_select(index)));
    }

    season::panel(trigger, menu, false)
}

fn menu_row_content<'a, Message>(
    value: Cow<'static, str>,
    selected: bool,
    count: Option<Cow<'static, str>>,
) -> iced::Element<'a, Message>
where
    Message: Clone + 'a,
{
    let mut row = Row::new()
        .push(internal::tick_glyph(selected))
        .push(value_text(value))
        .spacing(internal::TICK_GAP)
        .align_y(alignment::Vertical::Center);

    if let Some(count) = count {
        let count_text = text::micro_label(count)
            .font(font::medium())
            .color(color::TEXT_SECONDARY);
        row = row.push(Space::new().width(Length::Fill)).push(count_text);
    }

    row.width(Length::Fill).into()
}

/// Fixed trigger row width. Shapes the prefix label and every value in one
/// font lock, then adds the icon column and prefix gap before rounding up to
/// the nearest 10 px.
fn trigger_width(label: &str, items: &[ItemRow], icon: Option<&'static str>) -> f32 {
    let label_text = prefix_text(label);
    let value_texts: Vec<text::Text<'_>> = items
        .iter()
        .map(|item| trigger_value_text(&item.name))
        .collect();

    let widths =
        text::shape_widths(std::iter::once(&label_text).chain(value_texts.iter()));
    let label_width = widths[0];
    let max_value_width = widths[1..].iter().copied().fold(0.0_f32, f32::max);

    let icon_width = if icon.is_some() {
        TRIGGER_ICON_SIZE + ICON_LABEL_GAP
    } else {
        0.0
    };

    internal::round_up_10_min(
        icon_width + label_width + PREFIX_GAP + max_value_width,
        internal::TRIGGER_MIN_WIDTH,
    )
}

fn prefix_text<'a>(
    content: impl iced::widget::text::IntoFragment<'a>,
) -> text::Text<'a> {
    text::label(content, text::Variant::Alt).font(font::medium())
}

fn trigger_value_text<'a>(
    content: impl iced::widget::text::IntoFragment<'a>,
) -> text::Text<'a> {
    internal::trigger_main_text(content)
}

fn value_text<'a>(content: Cow<'static, str>) -> text::Text<'a> {
    text::card_title(content)
        .font(font::semibold())
        .inherit_color()
}
