use std::borrow::Cow;
use std::sync::OnceLock;

use iced::widget::{Text, row, svg, text};
use iced::{Center, Element};

use crate::{color, font, icon};

static TOMATO_SVG: &[u8] = include_bytes!("../../assets/misc/tomato.svg");
static TOMATO: OnceLock<svg::Handle> = OnceLock::new();

/// Media rating display.
pub fn rating<'a, Message>(
    stars: Option<&'a str>,
    tomato_score: Option<&'a str>,
) -> Element<'a, Message>
where
    Message: 'a,
{
    let mut row = row![];

    if let Some(stars) = stars {
        row = row.push(stars_fragment(stars));
    }

    if let Some(tomato) = tomato_score {
        row = row.push(tomato_fragment(tomato));
    }

    row.spacing(8).into()
}

fn stars_fragment<'a, Message>(stars: &'a str) -> Element<'a, Message>
where
    Message: 'a,
{
    let icon = icon::filled("star").size(18).color(color::WARNING);
    let rating = text_part(stars);
    let context = text_part("/ 10");

    row![icon, rating, context]
        .align_y(Center)
        .spacing(4)
        .into()
}

fn tomato_fragment<'a, Message>(score: &'a str) -> Element<'a, Message>
where
    Message: 'a,
{
    let tomato =
        TOMATO.get_or_init(|| svg::Handle::from_memory(Cow::Borrowed(TOMATO_SVG)));

    let icon = svg(tomato.clone()).width(18).height(18);
    let score = text_part(score);

    row![icon, score].align_y(Center).spacing(4).into()
}

fn text_part(display: &str) -> Text<'_> {
    text(display)
        .size(14)
        .color(color::TEXT_SECONDARY)
        .font(font::semibold())
}
