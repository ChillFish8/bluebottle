//! Shared private utilities for the dropdown family.

use iced::widget::text::IntoFragment;
use iced::widget::{Row, Space};
use iced::{Element, Length, Padding, alignment};

use crate::widget::clickable::{Clickable, clickable};
use crate::widget::{animated_tick, text};
use crate::{color, font};

pub(super) const ROW_RADIUS: f32 = 8.0;

pub(super) const ROW_PADDING: Padding = Padding {
    top: 6.0,
    right: 10.0,
    bottom: 6.0,
    left: 10.0,
};

pub(super) const MENU_ROW_SPACING: f32 = 4.0;

pub(super) const TICK_GAP: f32 = 8.0;

pub(super) const HEADER_PADDING: Padding = Padding {
    top: 4.0,
    right: 10.0,
    bottom: 8.0,
    left: 10.0,
};

const TICK_GLYPH_SIZE: f32 = 14.0;

/// Shared menu row chassis. Carries hover tint, radius, padding, and width.
/// Variants wrap and chain their own selected fill or ring, plus a
/// resting/selected text colour ease when they want one.
pub(super) fn row<'a, Message>(
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
        .radius(ROW_RADIUS)
        .padding(ROW_PADDING)
        .width(Length::Fill)
}

/// The animated check used in the left column of a menu row.
pub(super) fn tick_glyph<'a, Message: 'a>(selected: bool) -> Element<'a, Message> {
    animated_tick::animated_tick(selected, TICK_GLYPH_SIZE).into()
}

/// A pinned menu header. The two slots sit at opposite edges of the row with
/// a fill spacer between.
pub(super) fn header_row<'a, Message>(
    left: impl Into<Element<'a, Message>>,
    right: impl Into<Element<'a, Message>>,
) -> Element<'a, Message>
where
    Message: Clone + 'a,
{
    Row::new()
        .push(left)
        .push(Space::new().width(Length::Fill))
        .push(right)
        .padding(HEADER_PADDING)
        .align_y(alignment::Vertical::Center)
        .width(Length::Fill)
        .into()
}

/// A semibold main label, used for trigger names across every variant.
pub(super) fn trigger_main_text<'a>(content: impl IntoFragment<'a>) -> text::Text<'a> {
    text::label(content, text::Variant::Main).font(font::semibold())
}

/// A medium-weight caption, used for trigger count fragments.
pub(super) fn count_caption<'a>(content: impl IntoFragment<'a>) -> text::Text<'a> {
    text::caption(content).font(font::medium())
}

/// Widest shaped width across the given runs, under a single font lock.
pub(super) fn widest_shaped<'a, I>(runs: I) -> f32
where
    I: IntoIterator<Item = &'a text::Text<'a>>,
{
    text::shape_widest(runs)
}

/// Rounds a natural width up to the nearest 10 px so the trigger stays put
/// as the displayed value changes.
pub(super) fn round_up_10(width: f32) -> f32 {
    (width / 10.0).ceil() * 10.0
}

/// Like [`round_up_10`] but clamps the result to `min_width`. Prevents an
/// empty trigger from collapsing to a chevron-only pill.
pub(super) fn round_up_10_min(width: f32, min_width: f32) -> f32 {
    round_up_10(width).max(min_width)
}

/// Floor applied to season, labelled, and filter triggers so an empty value
/// list still renders as a recognisable pill. Source carries its own larger
/// floor in keeping with its chip aesthetic.
pub(super) const TRIGGER_MIN_WIDTH: f32 = 60.0;
