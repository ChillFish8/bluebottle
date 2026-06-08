//! Fact grid card. A bordered glass card laying labelled values out in a
//! configurable column grid. Each cell stacks an uppercase eyebrow label over
//! the value at the card title role.

use std::borrow::Cow;

use iced::widget::{Row, Space, column};
use iced::{Element, Length, padding};

use super::core::card;
use crate::widget::text::{self, Variant};
use crate::{color, spacing};

/// A single labelled value rendered as one cell in the grid.
pub struct FactEntry {
    label: Cow<'static, str>,
    value: Cow<'static, str>,
}

/// Builds a [`FactEntry`].
pub fn fact(
    label: impl Into<Cow<'static, str>>,
    value: impl Into<Cow<'static, str>>,
) -> FactEntry {
    FactEntry {
        label: label.into(),
        value: value.into(),
    }
}

/// Lays `entries` out in a `columns`-wide grid inside a bordered glass card.
/// The final row pads with empty cells when the entry count does not divide
/// evenly so column edges stay aligned with the rows above.
pub fn fact_grid<'a, Message>(
    columns: usize,
    entries: impl IntoIterator<Item = FactEntry>,
) -> Element<'a, Message>
where
    Message: 'a,
{
    let columns = columns.max(1);
    let entries: Vec<FactEntry> = entries.into_iter().collect();

    let mut rows = column![].spacing(spacing::GAP_14).width(Length::Fill);

    for chunk in entries.chunks(columns) {
        let mut row: Row<'_, Message> = Row::new().spacing(spacing::GAP_16);
        for entry in chunk {
            row = row.push(cell(&entry.label, entry.value.clone()));
        }
        for _ in chunk.len()..columns {
            row = row.push(Space::new().width(Length::Fill));
        }
        rows = rows.push(row);
    }

    card(rows)
        .padding(padding::all(spacing::PAD_16))
        .width(Length::Fill)
        .into()
}

fn cell<'a, Message>(label: &str, value: Cow<'static, str>) -> Element<'a, Message>
where
    Message: 'a,
{
    let label =
        text::eyebrow(label.to_uppercase(), Variant::Main).color(color::TEXT_SECONDARY);
    let value = text::card_title(value).color(color::TEXT_PRIMARY);

    column![label, value]
        .spacing(spacing::GAP_4)
        .width(Length::Fill)
        .into()
}
