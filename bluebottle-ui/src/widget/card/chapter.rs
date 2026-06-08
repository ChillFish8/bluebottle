//! Chapter Row. The list unit of a chapter-selection sheet. A timestamp on
//! the left in the accent colour, the chapter name in the middle, a chevron
//! on the right. The whole capsule dispatches a single press message.

use std::borrow::Cow;

use iced::widget::{Row, container};
use iced::{Element, Length, Padding, alignment};

use crate::widget::clickable::clickable;
use crate::widget::skeleton::skeleton;
use crate::widget::text;
use crate::{border, color, font, icon, spacing};

const CHEVRON_SIZE: f32 = 20.0;

const TIMESTAMP_BAR_MM_SS_WIDTH: f32 = 36.0;
const TIMESTAMP_BAR_HH_MM_SS_WIDTH: f32 = 56.0;
const TIMESTAMP_BAR_HEIGHT: f32 = 11.0;
const NAME_BAR_HEIGHT: f32 = 12.0;

/// Builds a Chapter Row.
pub fn chapter_row<Message>(
    start_seconds: u32,
    name: impl Into<Cow<'static, str>>,
) -> ChapterRow<Message> {
    ChapterRow {
        start_seconds,
        name: name.into(),
        show_hours: false,
        on_click: None,
    }
}

/// Builder for [`chapter_row`].
pub struct ChapterRow<Message> {
    start_seconds: u32,
    name: Cow<'static, str>,
    show_hours: bool,
    on_click: Option<Message>,
}

impl<Message> ChapterRow<Message> {
    /// Renders the timestamp as `HH:MM:SS` when `show` is true and as `MM:SS`
    /// otherwise. Set once per list based on the total runtime.
    pub fn show_hours(mut self, show: bool) -> Self {
        self.show_hours = show;
        self
    }

    /// Press anywhere on the row.
    pub fn on_click(mut self, message: Message) -> Self {
        self.on_click = Some(message);
        self
    }
}

impl<'a, Message> From<ChapterRow<Message>> for Element<'a, Message>
where
    Message: Clone + 'a,
{
    fn from(row: ChapterRow<Message>) -> Self {
        let ChapterRow {
            start_seconds,
            name,
            show_hours,
            on_click,
        } = row;

        let timestamp = text::caption(format_timestamp(start_seconds, show_hours))
            .font(font::semibold())
            .color(color::primary());

        let name_text = text::label(name, text::Variant::Main).width(Length::Fill);

        let chevron = icon::filled("arrow_right")
            .size(CHEVRON_SIZE)
            .color(color::TEXT_SECONDARY);

        let content = Row::new()
            .push(timestamp)
            .push(name_text)
            .push(chevron)
            .spacing(spacing::GAP_16)
            .align_y(alignment::Vertical::Center);

        clickable(content)
            .tint(color::border())
            .radius(border::ROUNDED_LG)
            .padding(Padding {
                top: spacing::PAD_8,
                right: spacing::PAD_12,
                bottom: spacing::PAD_8,
                left: spacing::PAD_12,
            })
            .width(Length::Fill)
            .on_press_maybe(on_click)
            .into()
    }
}

/// Shimmer placeholder matching the row layout. A short bar for the
/// timestamp, a full-width bar for the name, a small square for the chevron
/// slot. `show_hours` widens the timestamp bar to fit `HH:MM:SS` so the
/// chevron and name do not shift when the real chapters arrive.
pub fn chapter_row_skeleton<'a, Message>(show_hours: bool) -> Element<'a, Message>
where
    Message: Clone + 'a,
{
    let timestamp_width = if show_hours {
        TIMESTAMP_BAR_HH_MM_SS_WIDTH
    } else {
        TIMESTAMP_BAR_MM_SS_WIDTH
    };

    let timestamp_bar: Element<'a, Message> = skeleton()
        .width(Length::Fixed(timestamp_width))
        .height(Length::Fixed(TIMESTAMP_BAR_HEIGHT))
        .radius(border::ROUNDED_XS)
        .into();

    let name_bar: Element<'a, Message> = skeleton()
        .width(Length::Fill)
        .height(Length::Fixed(NAME_BAR_HEIGHT))
        .radius(border::ROUNDED_XS)
        .into();

    let chevron_dot: Element<'a, Message> = skeleton()
        .width(Length::Fixed(CHEVRON_SIZE))
        .height(Length::Fixed(CHEVRON_SIZE))
        .radius(border::ROUNDED_XS)
        .into();

    let content = Row::new()
        .push(timestamp_bar)
        .push(name_bar)
        .push(chevron_dot)
        .spacing(spacing::GAP_16)
        .align_y(alignment::Vertical::Center)
        .width(Length::Fill);

    container(content)
        .width(Length::Fill)
        .padding(Padding {
            top: spacing::PAD_8,
            right: spacing::PAD_12,
            bottom: spacing::PAD_8,
            left: spacing::PAD_12,
        })
        .into()
}

fn format_timestamp(seconds: u32, show_hours: bool) -> String {
    if show_hours {
        let h = seconds / 3600;
        let m = (seconds % 3600) / 60;
        let s = seconds % 60;
        format!("{h:02}:{m:02}:{s:02}")
    } else {
        let m = seconds / 60;
        let s = seconds % 60;
        format!("{m:02}:{s:02}")
    }
}
