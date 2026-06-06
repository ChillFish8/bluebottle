//! Continue-watching cards. Two layouts on the accent clickable card chassis.
//!
//! [continue_film] is a slim row pairing a play icon on the left with a label
//! and progress rail filling the rest of the card. [continue_show] swaps the
//! icon for an episode poster with a centered play icon, then names the next
//! episode and points the eye to it with a trailing chevron. Both carry a
//! determinate accent progress rail at the bottom of the body.

use std::borrow::Cow;
use std::time::Duration;

use iced::widget::{Space, column, container, row, stack};
use iced::{
    Background,
    Border,
    Center,
    Color,
    ContentFit,
    Element,
    Length,
    Theme,
    padding,
};

use super::core::clickable_card;
use crate::widget::ellipsis_text::ellipsis_text;
use crate::widget::image::Handle;
use crate::widget::media_image::media_image;
use crate::widget::skeleton::DEFAULT_RADIUS as IMAGE_RADIUS;
use crate::widget::spinner::{Tone, progress_rail};
use crate::widget::{separator, text};
use crate::{color, font, icon, style};

const CARD_RADIUS: f32 = 14.0;
const CARD_PADDING: f32 = 16.0;
const ROW_GAP: f32 = 16.0;
const STACK_GAP: f32 = 10.0;

const PLAY_GLYPH_SIZE: f32 = 32.0;

const POSTER_WIDTH: f32 = 144.0;
const POSTER_HEIGHT: f32 = 81.0;
const POSTER_SCRIM_ALPHA: f32 = 0.5;

const CHEVRON_SIZE: f32 = 24.0;

/// Continue-watching tile for a film. An accent card carrying a left-hand
/// accent play arrow, the resume label, and the remaining-time read-out, with
/// a determinate progress rail filling the bottom of the body.
pub fn continue_film<'a, Message>(
    elapsed: Duration,
    total: Duration,
    on_press: Option<Message>,
) -> Element<'a, Message>
where
    Message: Clone + 'a,
{
    let fraction = progress_fraction(elapsed, total);
    let remaining = total.saturating_sub(elapsed);

    let label = text::micro_label("CONTINUE WATCHING")
        .color(color::primary())
        .font(font::bold());

    let meta = row![
        text::micro_label(format_percent(fraction)).color(color::TEXT_SECONDARY),
        separator::inline_dot(),
        text::micro_label(format!("{} left", format_duration(remaining)))
            .color(color::TEXT_SECONDARY),
    ]
    .align_y(Center);

    let label_row = row![label, Space::new().width(Length::Fill), meta].align_y(Center);

    let body = column![label_row, accent_rail(fraction)]
        .spacing(STACK_GAP)
        .width(Length::Fill);

    let content = row![play_arrow(), body].spacing(ROW_GAP).align_y(Center);

    accent_card(content, on_press)
}

/// Continue-watching tile for a show. An accent card pairing an episode poster
/// with the season and episode ident, the episode title, and the remaining-time
/// read-out. A trailing accent chevron points to the next episode.
pub fn continue_show<'a, Message>(
    poster: Handle,
    season: u32,
    episode: u32,
    episode_name: impl Into<Cow<'a, str>>,
    elapsed: Duration,
    total: Duration,
    on_press: Option<Message>,
) -> Element<'a, Message>
where
    Message: Clone + 'a,
{
    let fraction = progress_fraction(elapsed, total);
    let remaining = total.saturating_sub(elapsed);

    let label = row![
        text::micro_label("CONTINUE WATCHING")
            .color(color::primary())
            .font(font::bold()),
        separator::inline_dot(),
        text::micro_label(format!("S{season} E{episode}"))
            .color(color::primary())
            .font(font::bold()),
    ]
    .align_y(Center);

    let title = ellipsis_text(text::body(episode_name.into()).font(font::semibold()))
        .width(Length::Fill);

    let time_left = text::caption(format!(
        "{} left of {}",
        format_duration(remaining),
        format_duration(total),
    ));

    let meta = column![label, title, time_left].spacing(0);

    let body = column![meta, accent_rail(fraction)]
        .spacing(STACK_GAP)
        .width(Length::Fill);

    let chevron = icon::filled("chevron_right")
        .size(CHEVRON_SIZE)
        .color(color::primary());

    let content = row![show_poster(poster), body, chevron]
        .spacing(ROW_GAP)
        .align_y(Center);

    accent_card(content, on_press)
}

fn accent_card<'a, Message>(
    content: impl Into<Element<'a, Message>>,
    on_press: Option<Message>,
) -> Element<'a, Message>
where
    Message: Clone + 'a,
{
    clickable_card(content)
        .background(color::primary_glass())
        .border(color::primary())
        .radius(CARD_RADIUS)
        .padding(padding::all(CARD_PADDING))
        .tint(color::hover_veil())
        .width(Length::Fill)
        .on_press_maybe(on_press)
        .into()
}

fn play_arrow<'a, Message>() -> Element<'a, Message>
where
    Message: 'a,
{
    icon::filled("play_arrow")
        .size(PLAY_GLYPH_SIZE)
        .color(color::primary())
        .into()
}

fn show_poster<'a, Message>(handle: Handle) -> Element<'a, Message>
where
    Message: Clone + 'a,
{
    let poster_image = iced::widget::image(handle)
        .width(POSTER_WIDTH)
        .height(POSTER_HEIGHT)
        .content_fit(ContentFit::Cover)
        .border_radius(IMAGE_RADIUS);

    let scrim = container(Space::new())
        .width(Length::Fill)
        .height(Length::Fill)
        .style(|_theme: &Theme| container::Style {
            background: Some(Background::Color(color::with_alpha(
                Color::BLACK,
                POSTER_SCRIM_ALPHA,
            ))),
            border: Border::default().rounded(IMAGE_RADIUS),
            ..container::Style::default()
        });

    let centered_play = container(play_arrow())
        .width(Length::Fill)
        .height(Length::Fill)
        .align_x(Center)
        .align_y(Center);

    let overlay = stack![scrim, centered_play];

    // media_image draws the overlay at rest when no on_press is set, sizes it
    // to the image, and shares the IMAGE_RADIUS rounding so the scrim, play
    // icon, and any future hover affordances trace the same shape. The
    // wrapping container exists only to cast the drop shadow underneath.
    let poster = media_image(poster_image).overlay(overlay);

    container(poster)
        .style(|_theme: &Theme| container::Style {
            border: Border::default().rounded(IMAGE_RADIUS),
            shadow: style::ELEVATION_INLINE,
            ..container::Style::default()
        })
        .into()
}

fn accent_rail<'a, Message>(fraction: f32) -> Element<'a, Message>
where
    Message: 'a,
{
    progress_rail()
        .value(fraction)
        .tone(Tone::Accent)
        .width(Length::Fill)
        .into()
}

/// Fraction of `total` that `elapsed` represents, clamped to `[0, 1]`. Returns
/// zero for a zero-length total rather than dividing by zero or pretending one
/// second is the minimum.
fn progress_fraction(elapsed: Duration, total: Duration) -> f32 {
    if total.is_zero() {
        return 0.0;
    }
    (elapsed.as_secs_f32() / total.as_secs_f32()).clamp(0.0, 1.0)
}

/// Percent string. Floors below 100 so the label never reads 100% while the
/// rail still has visible track remaining.
fn format_percent(fraction: f32) -> String {
    let percent = if fraction >= 1.0 {
        100
    } else {
        ((fraction * 100.0).floor() as u32).min(99)
    };
    format!("{percent}%")
}

/// Short human duration. Rounds partial minutes up so the last 59 seconds of
/// a film read as "1m" rather than collapsing to "0m" before the bar is full.
fn format_duration(d: Duration) -> String {
    let seconds = d.as_secs();
    if seconds == 0 {
        return "0m".into();
    }

    let total_minutes = seconds.div_ceil(60);
    let hours = total_minutes / 60;
    let minutes = total_minutes % 60;

    if hours == 0 {
        format!("{minutes}m")
    } else if minutes == 0 {
        format!("{hours}h")
    } else {
        format!("{hours}h {minutes}m")
    }
}
