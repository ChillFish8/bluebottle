//! Source / library selector. The trigger is a capsule chip with a glowing
//! status dot, the library glyph, and the active source name. Each menu row
//! carries the rich [`SourceEntry`] layout.

use std::borrow::Cow;

use iced::widget::{Row, Space, column, container, space};
use iced::{
    Background,
    Border,
    Color,
    Element,
    Length,
    Padding,
    Shadow,
    Vector,
    alignment,
};

use super::chassis::{Dropdown, dropdown};
use super::internal;
use crate::widget::clickable::Clickable;
use crate::widget::scrollable::scrollable;
use crate::widget::{button, text};
use crate::{border, color, font, icon, spacing};

const TRIGGER_PADDING: Padding = Padding {
    top: spacing::PAD_6,
    right: spacing::PAD_8,
    bottom: spacing::PAD_6,
    left: spacing::PAD_10,
};

const MENU_PADDING: Padding = Padding {
    top: spacing::PAD_6,
    right: spacing::PAD_6,
    bottom: spacing::PAD_6,
    left: spacing::PAD_6,
};

// Source rows are taller than the shared default so the rich layout
// breathes inside the menu.
const ROW_PADDING: Padding = Padding {
    top: spacing::PAD_10,
    right: spacing::PAD_12,
    bottom: spacing::PAD_10,
    left: spacing::PAD_12,
};

const STATUS_DOT_SIZE: f32 = 8.0;
const STATUS_DOT_GLOW_BLUR: f32 = 6.0;
const STATUS_DOT_GLOW_ALPHA: f32 = 0.6;
const LIBRARY_ICON_SIZE: f32 = 14.0;

const CHIP_PADDING: Padding = Padding {
    top: 1.0,
    right: spacing::PAD_6,
    bottom: 1.0,
    left: spacing::PAD_6,
};

// Greater than the 6 px STATUS_DOT_GLOW_BLUR so the dot's halo does not
// crowd the status caption.

const TRIGGER_MIN_WIDTH: f32 = 120.0;

const MENU_WIDTH: f32 = 320.0;
const MENU_MAX_HEIGHT: f32 = 320.0;

const FOOTER_PADDING: Padding = Padding {
    top: spacing::PAD_4,
    right: spacing::PAD_4,
    bottom: 2.0,
    left: spacing::PAD_4,
};

/// Connection state of a source. Drives the trigger dot colour and the row
/// status line. Offline is intentionally absent. A source that cannot answer
/// whether it holds the show has no place in the picker.
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum SourceStatus {
    Online,
    Downloaded,
}

impl SourceStatus {
    fn color(self) -> iced::Color {
        match self {
            SourceStatus::Online => color::success(),
            SourceStatus::Downloaded => color::primary(),
        }
    }

    fn label(self) -> &'static str {
        match self {
            SourceStatus::Online => "Online",
            SourceStatus::Downloaded => "Downloaded",
        }
    }
}

/// Where a source lives. Drives the eyebrow tag on each row.
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum SourceTag {
    Local,
    Recommended,
    Cast,
}

impl SourceTag {
    fn label(self) -> &'static str {
        match self {
            SourceTag::Local => "LOCAL",
            SourceTag::Recommended => "RECOMMENDED",
            SourceTag::Cast => "CAST",
        }
    }

    fn tint(self) -> Color {
        match self {
            SourceTag::Recommended => color::primary(),
            SourceTag::Local | SourceTag::Cast => color::TEXT_SECONDARY,
        }
    }
}

/// Media resolution the source serves at. Drives the right-column chip.
#[derive(Debug, Clone, Eq, PartialEq)]
pub enum Resolution {
    UHD4K,
    UHD4KHDR,
    FullHD,
    HD,
    SD,
    Other(Cow<'static, str>),
}

impl Resolution {
    fn label(&self) -> Cow<'static, str> {
        match self {
            Resolution::UHD4K => Cow::Borrowed("4K"),
            Resolution::UHD4KHDR => Cow::Borrowed("4K HDR"),
            Resolution::FullHD => Cow::Borrowed("1080P"),
            Resolution::HD => Cow::Borrowed("720P"),
            Resolution::SD => Cow::Borrowed("480P"),
            Resolution::Other(value) => value.clone(),
        }
    }

    fn tint(&self) -> Color {
        match self {
            Resolution::UHD4KHDR => color::GOLD,
            Resolution::UHD4K => color::primary(),
            _ => color::TEXT_SECONDARY,
        }
    }
}

/// One source's worth of menu data. Build with [`source_entry`].
#[derive(Clone)]
pub struct SourceEntry {
    name: Cow<'static, str>,
    status: SourceStatus,
    address: Cow<'static, str>,
    episode_count: u32,
    tags: Vec<SourceTag>,
    resolution: Resolution,
}

/// Builds a [`SourceEntry`]. Tags render as bordered glass pills in the
/// order given, accent for Recommended and secondary for the rest.
pub fn source_entry(
    name: impl Into<Cow<'static, str>>,
    status: SourceStatus,
    address: impl Into<Cow<'static, str>>,
    episode_count: u32,
    tags: impl IntoIterator<Item = SourceTag>,
    resolution: Resolution,
) -> SourceEntry {
    SourceEntry {
        name: name.into(),
        status,
        address: address.into(),
        episode_count,
        tags: tags.into_iter().collect(),
        resolution,
    }
}

/// A self-managing source dropdown. The trigger is a capsule chip and the
/// menu surfaces one row per entry. Wire optional extras through
/// [`Source::footer_action`] and [`Source::on_toggle`].
pub fn source<'a, Message>(
    entries: impl IntoIterator<Item = SourceEntry>,
    selected: usize,
    on_select: impl Fn(usize) -> Message + 'a,
) -> Source<'a, Message>
where
    Message: Clone + 'a,
{
    Source {
        entries: entries.into_iter().collect(),
        selected,
        on_select: Box::new(on_select),
        footer: None,
        on_toggle: None,
    }
}

/// A configured source dropdown built by [`source`].
pub struct Source<'a, Message> {
    entries: Vec<SourceEntry>,
    selected: usize,
    on_select: Box<dyn Fn(usize) -> Message + 'a>,
    footer: Option<FooterAction<Message>>,
    on_toggle: Option<Box<dyn Fn(bool) -> Message + 'a>>,
}

struct FooterAction<Message> {
    label: Cow<'static, str>,
    on_press: Message,
}

impl<'a, Message> Source<'a, Message>
where
    Message: Clone + 'a,
{
    /// Pins a ghost-button footer below the rows.
    pub fn footer_action(
        mut self,
        label: impl Into<Cow<'static, str>>,
        on_press: Message,
    ) -> Self {
        self.footer = Some(FooterAction {
            label: label.into(),
            on_press,
        });
        self
    }

    /// Forwards open and close events. Wiring this puts the chassis into
    /// controlled mode so the caller owns `expanded`.
    pub fn on_toggle(mut self, f: impl Fn(bool) -> Message + 'a) -> Self {
        self.on_toggle = Some(Box::new(f));
        self
    }
}

impl<'a, Message> From<Source<'a, Message>> for Dropdown<'a, Message>
where
    Message: Clone + 'a,
{
    fn from(src: Source<'a, Message>) -> Dropdown<'a, Message> {
        let trigger_width = trigger_width(&src.entries);
        let trigger = trigger_label(&src.entries, src.selected, trigger_width);

        let header = header_row(src.entries.len());

        let mut rows = column![]
            .spacing(internal::MENU_ROW_SPACING)
            .width(Length::Fill);
        for (index, entry) in src.entries.iter().enumerate() {
            let selected = index == src.selected;
            rows = rows.push(row(
                row_content::<Message>(entry, selected),
                selected,
                (src.on_select)(index),
            ));
        }

        let scroll = scrollable(rows)
            .max_height(MENU_MAX_HEIGHT)
            .fade_edges(color::GLASS_OPAQUE);

        let mut menu = column![header, scroll]
            .spacing(spacing::GAP_4)
            .width(Length::Fill);

        if let Some(footer) = src.footer {
            menu = menu.push(footer_row(footer));
        }

        let mut panel =
            panel(trigger, menu, false).menu_width(Length::Fixed(MENU_WIDTH));
        if let Some(handler) = src.on_toggle {
            panel = panel.on_toggle(handler);
        }
        panel
    }
}

impl<'a, Message> From<Source<'a, Message>> for Element<'a, Message>
where
    Message: Clone + 'a,
{
    fn from(src: Source<'a, Message>) -> Element<'a, Message> {
        Dropdown::from(src).into()
    }
}

/// The bare panel chassis for callers that need to compose a custom trigger
/// or menu over the same chip chrome.
pub fn panel<'a, Message>(
    label: impl Into<Element<'a, Message>>,
    menu: impl Into<Element<'a, Message>>,
    expanded: bool,
) -> Dropdown<'a, Message>
where
    Message: Clone + 'a,
{
    dropdown(label, menu, expanded)
        .radius(border::ROUNDED_FULL)
        .padding(TRIGGER_PADDING)
        .background(color::overlay_fill())
        .tint(color::overlay_fill())
        .border(color::border_strong())
        .selected_border(color::primary())
        .menu_background(color::GLASS_OPAQUE)
        .menu_border(color::border())
        .menu_radius(border::ROUNDED_2XL)
        .menu_padding(MENU_PADDING)
        .menu_width(Length::Fixed(MENU_WIDTH))
}

/// A source menu row. The selected row picks up both the accent fill and the
/// 1 px inset accent ring.
pub fn row<'a, Message>(
    content: impl Into<Element<'a, Message>>,
    selected: bool,
    on_press: Message,
) -> Clickable<'a, Message>
where
    Message: Clone + 'a,
{
    internal::row(content, selected, on_press)
        .padding(ROW_PADDING)
        .selected_background(color::accent_row_selected())
        .selected_border(color::primary())
}

fn trigger_label<'a, Message>(
    entries: &[SourceEntry],
    selected: usize,
    width: f32,
) -> Element<'a, Message>
where
    Message: Clone + 'a,
{
    let entry = entries.get(selected);
    let name: Cow<'static, str> = entry
        .map(|e| e.name.clone())
        .unwrap_or(Cow::Borrowed("No source"));

    let cast_icon = icon::filled("cast")
        .size(LIBRARY_ICON_SIZE)
        .color(color::TEXT_PRIMARY);

    let name_text = trigger_name_text(name);

    // Always reserve the dot slot so the trigger keeps the same width whether
    // or not a source is selected. When no entry is selected the slot stays
    // empty rather than rendering a meaningless dot.
    let dot: Element<'a, Message> = match entry {
        Some(entry) => status_dot(entry.status.color()),
        None => Space::new()
            .width(STATUS_DOT_SIZE)
            .height(STATUS_DOT_SIZE)
            .into(),
    };

    Row::new()
        .push(dot)
        .push(cast_icon)
        .push(name_text)
        .push(Space::new().width(Length::Fill))
        .spacing(spacing::GAP_6)
        .width(Length::Fixed(width))
        .align_y(alignment::Vertical::Center)
        .into()
}

fn trigger_name_text<'a>(
    content: impl iced::widget::text::IntoFragment<'a>,
) -> text::Text<'a> {
    internal::trigger_main_text(content)
}

/// Widest natural rendering rounded up to the nearest 10 px, clamped to
/// [`TRIGGER_MIN_WIDTH`] so the chip never collapses below a readable
/// minimum.
fn trigger_width(entries: &[SourceEntry]) -> f32 {
    let names: Vec<text::Text<'_>> = entries
        .iter()
        .map(|e| trigger_name_text(e.name.as_ref()))
        .collect();
    let name_width = internal::widest_shaped(names.iter());

    let chrome = STATUS_DOT_SIZE + LIBRARY_ICON_SIZE + (spacing::GAP_6 * 3.0);
    internal::round_up_10_min(name_width + chrome, TRIGGER_MIN_WIDTH)
}

fn row_content<'a, Message>(entry: &SourceEntry, selected: bool) -> Element<'a, Message>
where
    Message: Clone + 'a,
{
    let box_ =
        button::checkbox::<Message>(selected, button::CheckboxSizeVariant::Alt, None);

    let name = text::card_title(entry.name.clone())
        .font(font::semibold())
        .inherit_color();

    let resolution_chip = bordered_chip(
        text::micro_label(entry.resolution.label())
            .font(font::mono_medium())
            .color(entry.resolution.tint()),
        entry.resolution.tint(),
    );

    let name_row = Row::new()
        .push(name)
        .push(Space::new().width(Length::Fill))
        .push(resolution_chip)
        .spacing(spacing::GAP_8)
        .align_y(alignment::Vertical::Center)
        .width(Length::Fill);

    let status_text = text::caption(entry.status.label())
        .font(font::medium())
        .color(color::TEXT_SECONDARY);

    let mut status_row = Row::new()
        .push(status_dot(entry.status.color()))
        .push(status_text)
        .spacing(spacing::GAP_8)
        .align_y(alignment::Vertical::Center);

    if !entry.tags.is_empty() {
        status_row = status_row.push(
            text::caption("•")
                .font(font::medium())
                .color(color::TEXT_SECONDARY),
        )
    }

    for tag in &entry.tags {
        status_row = status_row.push(tag_chip(*tag));
    }

    let status_row = status_row.width(Length::Fill);

    let host_eps = text::caption(format!(
        "{} \u{00b7} {} eps",
        entry.address, entry.episode_count
    ))
    .font(font::mono_regular())
    .color(color::TEXT_SECONDARY);

    let middle = column![name_row, status_row, host_eps]
        .spacing(spacing::GAP_8)
        .width(Length::Fill);

    Row::new()
        .push(box_)
        .push(middle)
        .spacing(spacing::GAP_12)
        .width(Length::Fill)
        .align_y(alignment::Vertical::Center)
        .into()
}

/// A glowing coloured dot. A `tint` fill behind a centred shadow at 60
/// percent sRGB alpha so the halo rings all the way around.
fn status_dot<'a, Message: 'a>(tint: Color) -> Element<'a, Message> {
    container(space().width(STATUS_DOT_SIZE).height(STATUS_DOT_SIZE))
        .style(move |_theme| container::Style {
            background: Some(Background::Color(tint)),
            border: Border {
                radius: border::ROUNDED_FULL.into(),
                ..Border::default()
            },
            shadow: Shadow {
                color: color::with_alpha(tint, color::srgb_alpha(STATUS_DOT_GLOW_ALPHA)),
                offset: Vector::ZERO,
                blur_radius: STATUS_DOT_GLOW_BLUR,
            },
            ..container::Style::default()
        })
        .into()
}

/// A bordered glass pill. Tint at 28 percent sRGB behind a 1 px solid ring.
fn bordered_chip<'a, Message: 'a>(
    content: impl Into<Element<'a, Message>>,
    tint: Color,
) -> Element<'a, Message> {
    container(content)
        .padding(CHIP_PADDING)
        .style(move |_theme| container::Style {
            background: Some(Background::Color(color::with_alpha(
                tint,
                color::srgb_alpha(0.28),
            ))),
            border: Border {
                radius: border::ROUNDED_FULL.into(),
                width: 1.0,
                color: tint,
            },
            ..container::Style::default()
        })
        .into()
}

fn tag_chip<'a, Message: 'a>(tag: SourceTag) -> Element<'a, Message> {
    let tint = tag.tint();
    bordered_chip(
        text::eyebrow(tag.label(), text::Variant::Main)
            .font(font::bold())
            .color(tint),
        tint,
    )
}

fn header_row<'a, Message>(count: usize) -> Element<'a, Message>
where
    Message: Clone + 'a,
{
    let label =
        text::eyebrow("PLAY FROM", text::Variant::Main).color(color::TEXT_SECONDARY);
    let count = text::eyebrow(format!("{count} libraries"), text::Variant::Main)
        .color(color::TEXT_SECONDARY);

    internal::header_row(label, count)
}

fn footer_row<'a, Message>(footer: FooterAction<Message>) -> Element<'a, Message>
where
    Message: Clone + 'a,
{
    let FooterAction { label, on_press } = footer;
    let action = button::ghost_small(label, Some("storage"), on_press);

    Row::new()
        .push(action)
        .push(Space::new().width(Length::Fill))
        .padding(FOOTER_PADDING)
        .align_y(alignment::Vertical::Center)
        .width(Length::Fill)
        .into()
}
