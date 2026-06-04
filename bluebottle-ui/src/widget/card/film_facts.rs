//! Film facts card. A two-row, three-column grid of eyebrow-labelled facts.
//! Each cell pairs an uppercase muted label with the value below in the card
//! title role.

use std::borrow::Cow;

use iced::widget::{Row, column};
use iced::{Element, Length, padding};

use super::core::card;
use crate::widget::text::{self, Variant};
use crate::{color, font};

const ROW_SPACING: f32 = 14.0;
const COL_SPACING: f32 = 16.0;
const CELL_SPACING: f32 = 4.0;
const CARD_PADDING: f32 = 16.0;

/// The six facts shown on the grid. Pass each as an owned or borrowed string.
pub struct FilmFacts {
    pub director: Cow<'static, str>,
    pub studio: Cow<'static, str>,
    pub released: Cow<'static, str>,
    pub runtime: Cow<'static, str>,
    pub language: Cow<'static, str>,
    pub rating: Cow<'static, str>,
}

/// A bordered glass card laying [`FilmFacts`] out as a three by two grid.
pub fn film_facts<'a, Message>(facts: FilmFacts) -> Element<'a, Message>
where
    Message: 'a,
{
    let top = Row::new()
        .push(fact("Director", facts.director))
        .push(fact("Studio", facts.studio))
        .push(fact("Released", facts.released))
        .spacing(COL_SPACING);

    let bottom = Row::new()
        .push(fact("Runtime", facts.runtime))
        .push(fact("Language", facts.language))
        .push(fact("Rating", facts.rating))
        .spacing(COL_SPACING);

    let grid = column![top, bottom].spacing(ROW_SPACING);

    card(grid)
        .padding(padding::all(16.0))
        .width(Length::Fill)
        .into()
}

/// A single fact column. The label is uppercased and tinted muted, the value
/// rides at the card title role.
fn fact<'a, Message>(
    label: &'static str,
    value: Cow<'static, str>,
) -> Element<'a, Message>
where
    Message: 'a,
{
    let label = text::eyebrow(label.to_uppercase(), Variant::Alt);
    let value = text::card_title(value)
        .font(font::medium())
        .color(color::TEXT_PRIMARY);

    column![label, value]
        .spacing(CELL_SPACING)
        .width(Length::Fill)
        .into()
}
