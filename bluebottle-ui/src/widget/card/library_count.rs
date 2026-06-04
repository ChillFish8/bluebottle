//! Library count card. A bordered glass tile that pairs a coloured icon chip
//! with a localised count above the library name. Built on top of the base
//! [`card`] chassis so the chrome stays in step with the rest of the family.

use std::borrow::Cow;

use iced::widget::{Row, column, container};
use iced::{Background, Border, Center, Color, Element, Length, Theme, padding};

use super::core::card;
use crate::widget::text;
use crate::{border, color, icon};

const ICON_BOX_SIZE: f32 = 38.0;
const ICON_SIZE: f32 = 18.0;
const ROW_GAP: f32 = 12.0;
const STACK_SPACING: f32 = 2.0;

// Icon chip recipe. The chip is the only spot of colour on the tile so it
// rides at a stronger fill than a neutral glass surface.
const CHIP_FILL_ALPHA: f32 = 0.20;

/// A library count tile. The card chrome stays at the neutral bordered glass
/// default. `tone` paints the icon chip, `icon_name` selects a Material Icons
/// glyph, and `count` is shown in bold above `name` with thousands separators.
pub fn library_count<'a, Message>(
    name: impl Into<Cow<'static, str>>,
    tone: Color,
    icon_name: &'static str,
    count: u64,
) -> Element<'a, Message>
where
    Message: 'a,
{
    let chip = icon_chip(icon_name, tone);

    let count_text = text::heading_medium(format_thousands(count));
    let name_text = text::caption(name.into());

    let stack = column![count_text, name_text].spacing(STACK_SPACING);

    let body = Row::new()
        .push(chip)
        .push(stack)
        .spacing(ROW_GAP)
        .align_y(Center);

    card(body)
        .padding(padding::all(16.0))
        .width(Length::Fill)
        .into()
}

/// A fully rounded glass chip painted in `tone` housing the Material Icons
/// glyph at `name`. Sized to match the library count tile and reused by
/// sibling cards.
pub(super) fn icon_chip<'a, Message>(
    name: &'static str,
    tone: Color,
) -> Element<'a, Message>
where
    Message: 'a,
{
    let glyph = icon::filled(name).size(ICON_SIZE).color(tone);

    let chip_background = color::with_alpha(tone, color::srgb_alpha(CHIP_FILL_ALPHA));

    container(glyph)
        .width(Length::Fixed(ICON_BOX_SIZE))
        .height(Length::Fixed(ICON_BOX_SIZE))
        .align_x(Center)
        .align_y(Center)
        .style(move |_theme: &Theme| container::Style {
            background: Some(Background::Color(chip_background)),
            border: Border::default().rounded(border::ROUNDED_FULL),
            ..container::Style::default()
        })
        .into()
}

/// Inserts a comma between every three digits from the right. `1000` becomes
/// `1,000` and so on.
pub(super) fn format_thousands(count: u64) -> String {
    let raw = count.to_string();
    let bytes = raw.as_bytes();
    let mut out = String::with_capacity(raw.len() + raw.len() / 3);

    for (i, byte) in bytes.iter().enumerate() {
        if i > 0 && (bytes.len() - i).is_multiple_of(3) {
            out.push(',');
        }
        out.push(*byte as char);
    }

    out
}
