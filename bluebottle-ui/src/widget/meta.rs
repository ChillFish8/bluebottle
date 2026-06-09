//! Compact meta chips for badges, categories, and tags.
//!
//! Three rounded-rectangle forms that share the same footprint. An
//! [informational] wraps a micro-label run in a hairline border with no fill.
//! A [category] carries the active toggle pill's accent recipe at caption
//! size. A [tag] wears the bordered-glass icon's neutral fill at caption size
//! without the ring. Each becomes interactive the moment an `on_press` is
//! supplied.

use std::borrow::Cow;
use std::marker::PhantomData;
use std::sync::LazyLock;
use std::time::Duration;

use iced::widget::svg::Svg;
use iced::widget::text::IntoFragment;
use iced::widget::{row, svg};
use iced::{Center, Element, Length, padding};

use crate::util::format_duration_short;
use crate::widget::clickable::clickable;
use crate::widget::text::Variant;
use crate::widget::{separator, text};
use crate::{border, color, font, icon, spacing};

/// Shared chip padding. 4 vertical, 8 horizontal.
fn meta_padding() -> padding::Padding {
    padding::Padding::default()
        .vertical(spacing::PAD_4)
        .horizontal(spacing::PAD_8)
}

/// A hairline-bordered chip around a micro-label run. The transparent fill keeps
/// the chip quiet against any surface. Use for inline metadata badges.
pub fn informational<'a, Message>(
    label: impl IntoFragment<'a>,
    on_press: Option<Message>,
) -> Element<'a, Message>
where
    Message: Clone + 'a,
{
    clickable(text::micro_label(label).color(color::TEXT_SECONDARY))
        .padding(meta_padding())
        .radius(border::ROUNDED_SM)
        .border(color::border_strong())
        .tint(color::hover_veil())
        .on_press_maybe(on_press)
        .into()
}

/// A chip carrying the active toggle pill's recipe. A soft accent fill behind
/// accent-tinted caption text. Use for genre and category callouts.
pub fn category<'a, Message>(
    label: impl IntoFragment<'a>,
    on_press: Option<Message>,
) -> Element<'a, Message>
where
    Message: Clone + 'a,
{
    clickable(text::caption(label).color(color::primary()))
        .padding(meta_padding())
        .radius(border::ROUNDED_SM)
        .background(color::primary_glass())
        .tint(color::hover_veil())
        .on_press_maybe(on_press)
        .into()
}

/// A neutral bordered-glass chip without the ring. A white 6% fill behind a
/// white caption. Use for free-form tags and detail-row chips.
pub fn tag<'a, Message>(
    label: impl IntoFragment<'a>,
    on_press: Option<Message>,
) -> Element<'a, Message>
where
    Message: Clone + 'a,
{
    clickable(text::caption(label).color(color::TEXT_PRIMARY))
        .padding(meta_padding())
        .radius(border::ROUNDED_SM)
        .background(color::border())
        .tint(color::hover_veil())
        .on_press_maybe(on_press)
        .into()
}

/// A full-pill frosted chip for use over imagery. White at 14% behind a
/// hairline at 16%, with a medium-weight micro label. Static. The backdrop
/// blur comes from the host [`media_image`](crate::widget::media_image),
/// so the chip itself only paints the fill, border, and label.
pub fn frosted<'a, Message>(label: impl IntoFragment<'a>) -> Element<'a, Message>
where
    Message: Clone + 'a,
{
    let fill = color::with_alpha(color::WHITE, color::srgb_alpha(0.14));
    let border = color::with_alpha(color::WHITE, color::srgb_alpha(0.16));

    clickable(
        text::micro_label(label)
            .font(font::medium())
            .letter_spacing(0.0)
            .color(color::TEXT_PRIMARY),
    )
    .padding(padding::Padding::default().vertical(5).horizontal(9))
    .background(fill)
    .border(border)
    .into()
}

/// Composed facts row in a fixed order. Year, gold-star rating, red-tomato
/// critic score, runtime, and the rating chip, strung together by the inline
/// metadata dot. Icons carry the only colour. Text rides on
/// [`color::TEXT_SECONDARY`] so the title above stays dominant. Builder
/// fields are independently optional. Omitted fields drop out of the line
/// with no separator.
///
/// `variant` selects the typography. [`Variant::Main`] uses the standard
/// [`text::label`] run. [`Variant::Alt`] uses the heavier
/// [`text::card_title`] run for hero placements.
pub fn metadata_line<'a, Message>(variant: Variant) -> MetadataLine<'a, Message>
where
    Message: Clone + 'a,
{
    MetadataLine {
        variant,
        year: None,
        star: None,
        tomato: None,
        runtime: None,
        rating: None,
        _phantom: PhantomData,
    }
}

/// A composed metadata line, built by [`metadata_line`].
pub struct MetadataLine<'a, Message> {
    variant: Variant,
    year: Option<u16>,
    star: Option<f32>,
    tomato: Option<u32>,
    runtime: Option<Duration>,
    rating: Option<Cow<'a, str>>,
    _phantom: PhantomData<Message>,
}

impl<'a, Message> MetadataLine<'a, Message>
where
    Message: Clone + 'a,
{
    /// Sets the release year.
    pub fn year(mut self, year: u16) -> Self {
        self.year = Some(year);
        self
    }

    /// Sets the gold-star score. Formatted with one decimal place.
    pub fn star(mut self, value: f32) -> Self {
        self.star = Some(value);
        self
    }

    /// Sets the red-tomato critic score as a percentage.
    pub fn tomato(mut self, percent: u32) -> Self {
        self.tomato = Some(percent);
        self
    }

    /// Sets the runtime. Formatted as `Xh Ym`.
    pub fn runtime(mut self, duration: Duration) -> Self {
        self.runtime = Some(duration);
        self
    }

    /// Sets the age-rating chip label. Rendered as an [`informational`]
    /// chip at the tail of the line.
    pub fn rating(mut self, label: impl Into<Cow<'a, str>>) -> Self {
        self.rating = Some(label.into());
        self
    }
}

impl<'a, Message> From<MetadataLine<'a, Message>> for Element<'a, Message>
where
    Message: Clone + 'a,
{
    fn from(line: MetadataLine<'a, Message>) -> Self {
        let variant = line.variant;
        let mut items: Vec<Element<'a, Message>> = Vec::new();

        let icon_size = fact_icon_size(variant);

        fn push_dot<'a, M: 'a>(items: &mut Vec<Element<'a, M>>) {
            if !items.is_empty() {
                items.push(separator::inline_dot_lg().into());
            }
        }

        if let Some(year) = line.year {
            push_dot(&mut items);
            items.push(fact_text(variant, year.to_string()).into());
        }

        if let Some(value) = line.star {
            push_dot(&mut items);
            items.push(
                row![
                    icon::filled("star").size(icon_size).color(color::GOLD),
                    fact_text(variant, format!("{value:.1}")),
                ]
                .spacing(spacing::GAP_4)
                .align_y(Center)
                .into(),
            );
        }

        if let Some(percent) = line.tomato {
            push_dot(&mut items);
            items.push(
                row![
                    tomato_icon(icon_size),
                    fact_text(variant, format!("{percent}%"))
                ]
                .spacing(spacing::GAP_4)
                .align_y(Center)
                .into(),
            );
        }

        if let Some(duration) = line.runtime {
            push_dot(&mut items);
            items.push(fact_text(variant, format_duration_short(duration)).into());
        }

        if let Some(rating) = line.rating {
            push_dot(&mut items);
            items.push(informational(rating, None));
        }

        row(items).align_y(Center).into()
    }
}

static TOMATO_HANDLE: LazyLock<svg::Handle> = LazyLock::new(|| {
    svg::Handle::from_memory(include_bytes!("../../assets/misc/tomato.svg").as_slice())
});

fn fact_text<'a>(variant: Variant, content: impl IntoFragment<'a>) -> text::Text<'a> {
    match variant {
        Variant::Main => text::label(content, Variant::Alt),
        Variant::Alt => text::card_title(content),
    }
}

fn fact_icon_size(variant: Variant) -> f32 {
    match variant {
        Variant::Main => 12.0,
        Variant::Alt => 13.0,
    }
}

fn tomato_icon<'a>(size: f32) -> Svg<'a> {
    Svg::new(TOMATO_HANDLE.clone())
        .width(Length::Fixed(size))
        .height(Length::Fixed(size))
}

/// A small accent-tint pill that labels a carousel section. An accent 10%
/// fill behind an accent 20% hairline, with a bold caption-sized accent
/// label and an optional 11px leading glyph. The flat tint sets it apart
/// from [`frosted`], which lives over imagery.
pub fn section_badge<'a, Message>(
    label: impl IntoFragment<'a>,
    icon_name: Option<&'a str>,
    on_press: Option<Message>,
) -> Element<'a, Message>
where
    Message: Clone + 'a,
{
    let accent = color::primary();
    let fill = color::with_alpha(accent, color::srgb_alpha(0.10));
    let border = color::with_alpha(accent, color::srgb_alpha(0.20));

    let mut items = row![].spacing(spacing::GAP_4).align_y(Center);
    if let Some(name) = icon_name {
        items = items.push(icon::filled(name).size(11).color(accent));
    }
    items = items.push(text::caption(label).font(font::bold()).color(accent));

    clickable(items)
        .padding(padding::Padding::default().vertical(3).horizontal(8))
        .background(fill)
        .border(border)
        .tint(color::hover_veil())
        .on_press_maybe(on_press)
        .into()
}
