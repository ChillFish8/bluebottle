//! Search result card. The single best-match panel for a query.
//!
//! [top_result], [top_result_episode], and [top_result_album] are three
//! shapes of the same accent-ringed glass card. The artwork leads in its
//! native ratio at a shared height, then the body stacks a TOP RESULT
//! eyebrow, the matched title, a metadata line, and the full hero action
//! row. The accent border is reserved for this card alone so the
//! best-match read stays unique.

use std::borrow::Cow;
use std::time::Duration;

use iced::widget::{column, container, row};
use iced::{
    Background,
    Border,
    Center,
    Color,
    Element,
    Length,
    Shadow,
    Theme,
    Vector,
    padding,
};

use crate::widget::blur::Backdrop;
use crate::widget::button;
use crate::widget::media_image::media_image;
use crate::widget::meta::{metadata_line, section_badge};
use crate::widget::text::{self, Variant};
use crate::{border, color, font, spacing, style};

/// All three artwork shapes share a height so the card body keeps the same
/// vertical rhythm. The width varies with the native ratio of the source.
const ARTWORK_HEIGHT: f32 = 156.0;
const POSTER_WIDTH: f32 = 110.0;
const EPISODE_WIDTH: f32 = 277.0;
const ALBUM_WIDTH: f32 = ARTWORK_HEIGHT;
const ARTWORK_RADIUS: f32 = border::ROUNDED_LG;

const TOP_RESULT_PADDING: f32 = spacing::PAD_16;
const TOP_RESULT_RADIUS: f32 = border::ROUNDED_3XL;

/// Search result card framed by the only accent border in the system. An
/// indigo edge over a deep secondary glass marks the single best match for
/// the query. The 2:3 poster leads, the body stacks an eyebrow, the title
/// with the matched run highlighted, a metadata line, and the hero action
/// row.
pub fn top_result<'a, Message>(
    poster: Backdrop,
    title: impl Into<Cow<'a, str>>,
) -> TopResult<'a, Message>
where
    Message: Clone + 'a,
{
    new_top_result(poster, title, POSTER_WIDTH)
}

/// Episode sibling of [`top_result`]. A 16:9 landscape still leads the card
/// in place of the poster. Use for episodes and clips.
pub fn top_result_episode<'a, Message>(
    thumbnail: Backdrop,
    title: impl Into<Cow<'a, str>>,
) -> TopResult<'a, Message>
where
    Message: Clone + 'a,
{
    new_top_result(thumbnail, title, EPISODE_WIDTH)
}

/// Album sibling of [`top_result`]. A 1:1 square artwork leads the card in
/// place of the poster. Use for albums and singles.
pub fn top_result_album<'a, Message>(
    artwork: Backdrop,
    title: impl Into<Cow<'a, str>>,
) -> TopResult<'a, Message>
where
    Message: Clone + 'a,
{
    new_top_result(artwork, title, ALBUM_WIDTH)
}

fn new_top_result<'a, Message>(
    artwork: Backdrop,
    title: impl Into<Cow<'a, str>>,
    artwork_width: f32,
) -> TopResult<'a, Message>
where
    Message: Clone + 'a,
{
    TopResult {
        artwork,
        artwork_width,
        title: title.into(),
        group: None,
        highlight: None,
        year: None,
        star: None,
        tomato: None,
        runtime: None,
        rating: None,
        bookmarked: false,
        favourited: false,
        on_resume: None,
        on_details: None,
        on_add: None,
        on_bookmark: None,
        on_favourite: None,
    }
}

/// A composed best-match search card, built by [`top_result`],
/// [`top_result_episode`], or [`top_result_album`].
pub struct TopResult<'a, Message> {
    artwork: Backdrop,
    artwork_width: f32,
    title: Cow<'a, str>,
    group: Option<(Cow<'a, str>, &'a str)>,
    highlight: Option<Cow<'a, str>>,
    year: Option<u16>,
    star: Option<f32>,
    tomato: Option<u32>,
    runtime: Option<Duration>,
    rating: Option<Cow<'a, str>>,
    bookmarked: bool,
    favourited: bool,
    on_resume: Option<Message>,
    on_details: Option<Message>,
    on_add: Option<Message>,
    on_bookmark: Option<Message>,
    on_favourite: Option<Message>,
}

impl<'a, Message> TopResult<'a, Message>
where
    Message: Clone + 'a,
{
    /// Group label and leading glyph rendered as a [`section_badge`] beside
    /// the eyebrow. Use to name the content type. Film, Show, Episode, or
    /// Music.
    pub fn group(mut self, label: impl Into<Cow<'a, str>>, icon_name: &'a str) -> Self {
        self.group = Some((label.into(), icon_name));
        self
    }

    /// Substring of the title to wrap in the accent highlight mark. Matching
    /// is ASCII case-insensitive so the original cased title shows through.
    pub fn highlight(mut self, query: impl Into<Cow<'a, str>>) -> Self {
        self.highlight = Some(query.into());
        self
    }

    /// Sets the release year.
    pub fn year(mut self, year: u16) -> Self {
        self.year = Some(year);
        self
    }

    /// Sets the gold-star score.
    pub fn star(mut self, value: f32) -> Self {
        self.star = Some(value);
        self
    }

    /// Sets the red-tomato critic percentage.
    pub fn tomato(mut self, percent: u32) -> Self {
        self.tomato = Some(percent);
        self
    }

    /// Sets the runtime.
    pub fn runtime(mut self, duration: Duration) -> Self {
        self.runtime = Some(duration);
        self
    }

    /// Sets the age-rating chip label.
    pub fn rating(mut self, label: impl Into<Cow<'a, str>>) -> Self {
        self.rating = Some(label.into());
        self
    }

    /// Renders the bookmark icon in its accent on state.
    pub fn bookmarked(mut self, bookmarked: bool) -> Self {
        self.bookmarked = bookmarked;
        self
    }

    /// Renders the heart icon in its accent on state.
    pub fn favourited(mut self, favourited: bool) -> Self {
        self.favourited = favourited;
        self
    }

    /// Press of the solid Resume hero button.
    pub fn on_resume(mut self, message: Message) -> Self {
        self.on_resume = Some(message);
        self
    }

    /// Press of the ghost Details pill.
    pub fn on_details(mut self, message: Message) -> Self {
        self.on_details = Some(message);
        self
    }

    /// Press of the add icon button.
    pub fn on_add(mut self, message: Message) -> Self {
        self.on_add = Some(message);
        self
    }

    /// Press of the bookmark icon button.
    pub fn on_bookmark(mut self, message: Message) -> Self {
        self.on_bookmark = Some(message);
        self
    }

    /// Press of the heart icon button.
    pub fn on_favourite(mut self, message: Message) -> Self {
        self.on_favourite = Some(message);
        self
    }
}

impl<'a, Message> From<TopResult<'a, Message>> for Element<'a, Message>
where
    Message: Clone + 'a,
{
    fn from(card: TopResult<'a, Message>) -> Self {
        let TopResult {
            artwork,
            artwork_width,
            title,
            group,
            highlight,
            year,
            star,
            tomato,
            runtime,
            rating,
            bookmarked,
            favourited,
            on_resume,
            on_details,
            on_add,
            on_bookmark,
            on_favourite,
        } = card;

        let artwork = container(
            media_image(artwork)
                .width(artwork_width)
                .height(ARTWORK_HEIGHT)
                .corner_radius(ARTWORK_RADIUS),
        )
        .style(|_theme: &Theme| container::Style {
            border: Border::default().rounded(ARTWORK_RADIUS),
            shadow: style::ELEVATION_INLINE,
            ..container::Style::default()
        });

        let eyebrow = text::eyebrow("TOP RESULT", Variant::Main).font(font::bold());

        let header: Element<'a, Message> = if let Some((label, icon_name)) = group {
            row![eyebrow, section_badge(label, Some(icon_name), None)]
                .spacing(spacing::GAP_8)
                .align_y(Center)
                .into()
        } else {
            eyebrow.into()
        };

        let mut title = text::heading_medium(title);
        if let Some(query) = highlight {
            title = title.highlight(query);
        }

        let mut meta = metadata_line(Variant::Alt);
        if let Some(year) = year {
            meta = meta.year(year);
        }
        if let Some(value) = star {
            meta = meta.star(value);
        }
        if let Some(percent) = tomato {
            meta = meta.tomato(percent);
        }
        if let Some(duration) = runtime {
            meta = meta.runtime(duration);
        }
        if let Some(rating) = rating {
            meta = meta.rating(rating);
        }

        let actions = action_row(
            on_resume,
            on_details,
            on_add,
            on_bookmark,
            bookmarked,
            on_favourite,
            favourited,
        );

        let body = column![header, title, meta, actions]
            .spacing(spacing::GAP_8)
            .width(Length::Fill);

        let content = row![artwork, body].spacing(spacing::GAP_16).align_y(Center);

        container(content)
            .padding(padding::all(TOP_RESULT_PADDING))
            .width(Length::Fill)
            .style(|_theme: &Theme| container::Style {
                background: Some(Background::Color(color::with_alpha(
                    color::SECONDARY,
                    color::srgb_alpha(0.60),
                ))),
                border: Border {
                    radius: TOP_RESULT_RADIUS.into(),
                    width: 1.0,
                    color: color::with_alpha(color::primary(), color::srgb_alpha(0.40)),
                },
                shadow: Shadow {
                    color: color::with_alpha(Color::BLACK, 0.50),
                    offset: Vector::new(0.0, 20.0),
                    blur_radius: 40.0,
                },
                ..container::Style::default()
            })
            .into()
    }
}

fn action_row<'a, Message>(
    on_resume: Option<Message>,
    on_details: Option<Message>,
    on_add: Option<Message>,
    on_bookmark: Option<Message>,
    bookmarked: bool,
    on_favourite: Option<Message>,
    favourited: bool,
) -> Element<'a, Message>
where
    Message: Clone + 'a,
{
    let mut items = row![].spacing(spacing::GAP_8).align_y(Center);

    if let Some(message) = on_resume {
        items = items.push(button::hero("play_arrow", "Resume", message));
    }

    if let Some(message) = on_details {
        items = items.push(button::ghost("Details", Some("more_horiz"), message));
    }

    if let Some(message) = on_add {
        items = items.push(button::icon(
            "add",
            button::IconSizeVariant::Main,
            false,
            message,
        ));
    }

    if let Some(message) = on_bookmark {
        items = items.push(button::icon(
            "bookmark",
            button::IconSizeVariant::Main,
            bookmarked,
            message,
        ));
    }

    if let Some(message) = on_favourite {
        items = items.push(button::icon(
            "favorite",
            button::IconSizeVariant::Main,
            favourited,
            message,
        ));
    }

    items.into()
}
