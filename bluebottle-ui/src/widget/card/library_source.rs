//! Library source card. A bordered glass row pairing a source's leading
//! icon chip, identity, type, address, and status on the left with simple
//! per-category counts, a settings affordance, and a grab handle on the
//! right.

use std::borrow::Cow;

use iced::widget::{Row, Space, column, container, space};
use iced::{
    Alignment,
    Background,
    Border,
    Center,
    Color,
    Element,
    Length,
    Padding,
    Theme,
};

use super::core::card;
use super::library_count::{format_thousands, icon_chip};
use crate::widget::{button, text};
use crate::{border, color, font, icon, spacing};

const CARD_PADDING: Padding = Padding {
    top: spacing::PAD_14,
    right: spacing::PAD_14,
    bottom: spacing::PAD_14,
    left: spacing::PAD_14,
};

const STATUS_DOT_SIZE: f32 = 8.0;

const TYPE_CHIP_PADDING: Padding = Padding {
    top: spacing::PAD_2,
    right: spacing::PAD_8,
    bottom: spacing::PAD_2,
    left: spacing::PAD_8,
};
const TYPE_CHIP_FILL_ALPHA: f32 = 0.04;

const GRAB_ICON_SIZE: f32 = 20.0;

/// Where the source's media lives.
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum LibrarySourceKind {
    Remote,
    Local,
}

impl LibrarySourceKind {
    fn label(self) -> &'static str {
        match self {
            LibrarySourceKind::Remote => "REMOTE",
            LibrarySourceKind::Local => "LOCAL",
        }
    }
}

/// Reachability of the source. Drives the colour of the status dot and text.
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum LibrarySourceStatus {
    Online,
    Offline,
}

impl LibrarySourceStatus {
    fn tint(self) -> Color {
        match self {
            LibrarySourceStatus::Online => color::success(),
            LibrarySourceStatus::Offline => color::error(),
        }
    }

    fn label(self) -> &'static str {
        match self {
            LibrarySourceStatus::Online => "Online",
            LibrarySourceStatus::Offline => "Offline",
        }
    }
}

/// One per-category count shown on the right side of the card. Renders as a
/// bold localised number above the category label.
pub struct LibrarySourceCount {
    label: Cow<'static, str>,
    count: u64,
}

/// Builds a [`LibrarySourceCount`].
pub fn library_source_count(
    label: impl Into<Cow<'static, str>>,
    count: u64,
) -> LibrarySourceCount {
    LibrarySourceCount {
        label: label.into(),
        count,
    }
}

/// A library source card. The left side shows the leading icon chip, name,
/// type chip, address, and status. The right side carries the per-category
/// counts, a bordered glass settings button, and a grab handle icon.
pub fn library_source<'a, Message>(
    name: impl Into<Cow<'static, str>>,
    address: impl Into<Cow<'static, str>>,
    kind: LibrarySourceKind,
    status: LibrarySourceStatus,
    counts: impl IntoIterator<Item = LibrarySourceCount>,
    on_settings: Message,
) -> Element<'a, Message>
where
    Message: Clone + 'a,
{
    let (chip_icon, chip_tone) = leading_chip(kind, status);
    let chip = icon_chip(chip_icon, chip_tone);

    let identity = identity_block(name.into(), address.into(), kind, status);
    let right = right_side(counts, on_settings);

    let body = Row::new()
        .push(chip)
        .push(identity)
        .push(Space::new().width(Length::Fill))
        .push(right)
        .spacing(spacing::GAP_16)
        .align_y(Center);

    card(body).padding(CARD_PADDING).width(Length::Fill).into()
}

/// Picks the leading chip's icon and tone. Remote sources track the
/// connectivity status, local sources always ride the accent.
fn leading_chip(
    kind: LibrarySourceKind,
    status: LibrarySourceStatus,
) -> (&'static str, Color) {
    match (kind, status) {
        (LibrarySourceKind::Remote, LibrarySourceStatus::Online) => {
            ("cast_connected", color::success())
        },
        (LibrarySourceKind::Remote, LibrarySourceStatus::Offline) => {
            ("cast", color::error())
        },
        (LibrarySourceKind::Local, _) => ("folder_copy", color::primary()),
    }
}

fn identity_block<'a, Message>(
    name: Cow<'static, str>,
    address: Cow<'static, str>,
    kind: LibrarySourceKind,
    status: LibrarySourceStatus,
) -> Element<'a, Message>
where
    Message: Clone + 'a,
{
    let name_cluster = Row::new()
        .push(text::title_small(name))
        .push(type_chip(kind))
        .spacing(spacing::GAP_8)
        .align_y(Center);

    let status_cluster = Row::new()
        .push(status_dot(status.tint()))
        .push(text::caption(status.label()).color(status.tint()))
        .spacing(spacing::GAP_6)
        .align_y(Center);

    let name_row = Row::new()
        .push(name_cluster)
        .push(status_cluster)
        .spacing(spacing::GAP_12)
        .align_y(Center);

    column![name_row, text::caption(address)]
        .spacing(spacing::GAP_4)
        .into()
}

fn right_side<'a, Message>(
    counts: impl IntoIterator<Item = LibrarySourceCount>,
    on_settings: Message,
) -> Element<'a, Message>
where
    Message: Clone + 'a,
{
    let mut counts_row = Row::new().spacing(spacing::GAP_16).align_y(Center);
    for entry in counts {
        counts_row = counts_row.push(count_tile(entry.label, entry.count));
    }

    let settings = button::icon_flat("settings", false, Some(on_settings));
    let grab = icon::filled("drag_indicator")
        .size(GRAB_ICON_SIZE)
        .color(color::TEXT_SECONDARY);

    Row::new()
        .push(counts_row)
        .push(settings)
        .push(grab)
        .spacing(spacing::GAP_16)
        .align_y(Center)
        .into()
}

fn count_tile<'a, Message: 'a>(
    label: Cow<'static, str>,
    count: u64,
) -> Element<'a, Message> {
    let number = text::card_title(format_thousands(count))
        .font(font::semibold())
        .color(color::TEXT_PRIMARY);
    let label = text::caption(label);

    column![number, label]
        .spacing(spacing::GAP_2)
        .align_x(Alignment::End)
        .into()
}

fn status_dot<'a, Message: 'a>(tint: Color) -> Element<'a, Message> {
    container(space().width(STATUS_DOT_SIZE).height(STATUS_DOT_SIZE))
        .style(move |_theme: &Theme| container::Style {
            background: Some(Background::Color(tint)),
            border: Border::default().rounded(border::ROUNDED_FULL),
            ..container::Style::default()
        })
        .into()
}

fn type_chip<'a, Message: 'a>(kind: LibrarySourceKind) -> Element<'a, Message> {
    let label =
        text::eyebrow(kind.label(), text::Variant::Alt).color(color::TEXT_SECONDARY);

    container(label)
        .padding(TYPE_CHIP_PADDING)
        .style(move |_theme: &Theme| container::Style {
            background: Some(Background::Color(color::with_alpha(
                color::WHITE,
                color::srgb_alpha(TYPE_CHIP_FILL_ALPHA),
            ))),
            border: Border {
                radius: border::ROUNDED_FULL.into(),
                width: 1.0,
                color: color::border_strong(),
            },
            ..container::Style::default()
        })
        .into()
}
