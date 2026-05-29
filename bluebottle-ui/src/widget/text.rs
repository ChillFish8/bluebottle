use iced::widget::{Text, text};

use crate::{color, font};

/// Display Title Large
///
/// Hero & player titles. One per screen. The single largest thing in view. Tight tracking, line-height near 1.
pub fn display_large(input: &str) -> Text<'_> {
    // TODO: Handle the letter spacing.
    text(input)
        .font(font::bold())
        .size(52)
        .line_height(1.5)
        .color(color::TEXT_PRIMARY)
}

/// Display Title Medium
///
/// Hero & player titles. One per screen. The single largest thing in view. Tight tracking, line-height near 1.
pub fn display_medium(input: &str) -> Text<'_> {
    // TODO: Handle the letter spacing.
    text(input)
        .font(font::bold())
        .size(44)
        .line_height(1.5)
        .color(color::TEXT_PRIMARY)
}

/// Heading Large
///
/// Drawer tiles, episode names, top search result. The largest text inside a panel or overlay.
pub fn heading_large(input: &str) -> Text<'_> {
    // TODO: Handle the letter spacing.
    text(input)
        .font(font::bold())
        .size(26)
        .line_height(1.15)
        .color(color::TEXT_PRIMARY)
}

/// Header Medium
///
/// Drawer tiles, episode names, top search result. The largest text inside a panel or overlay.
pub fn heading_medium(input: &str) -> Text<'_> {
    // TODO: Handle the letter spacing.
    text(input)
        .font(font::bold())
        .size(22)
        .line_height(1.2)
        .color(color::TEXT_PRIMARY)
}

/// Title Small
///
/// Drawer tiles, episode names, top search result. The largest text inside a panel or overlay.
pub fn title_small(input: &str) -> Text<'_> {
    // TODO: Handle the letter spacing.
    text(input)
        .font(font::bold())
        .size(18)
        .color(color::TEXT_PRIMARY)
}

#[derive(Debug, Eq, PartialEq, Copy, Clone, Default)]
/// The context decides what variant the text should form to if
/// the given text has alt variants.
pub enum Variant {
    #[default]
    /// Primary format.
    Main,
    /// Secondary format.
    Alt,
}

/// Subtitle
///
/// Supporting lines beneath a title, search input, and reading copy. Lighter weights, relaxed line-height.
pub fn subtitle(input: &str, ctx: Variant) -> Text<'_> {
    let color = match ctx {
        Variant::Main => color::TEXT_PRIMARY,
        Variant::Alt => color::TEXT_SECONDARY,
    };

    text(input)
        .font(font::medium())
        .size(18)
        .line_height(1.4)
        .color(color)
}

/// Lead
///
/// Supporting lines beneath a title, search input, and reading copy. Lighter weights, relaxed line-height.
pub fn lead(input: &str, ctx: Variant) -> Text<'_> {
    let color = match ctx {
        Variant::Main => color::with_alpha(color::TEXT_PRIMARY, 0.78),
        Variant::Alt => color::TEXT_SECONDARY,
    };

    text(input)
        .font(font::regular())
        .size(16)
        .line_height(1.6)
        .color(color)
}

/// Body
pub fn body(input: &str) -> Text<'_> {
    text(input)
        .font(font::regular())
        .size(14)
        .line_height(1.6)
        .color(color::with_alpha(color::TEXT_PRIMARY, 0.82))
}

/// Section Heading
///
/// Supporting lines beneath a title, search input, and reading copy. Lighter weights, relaxed line-height.
pub fn section_heading(input: &str) -> Text<'_> {
    // TODO: Handle the letter spacing.
    text(input)
        .font(font::bold())
        .size(14)
        .color(color::TEXT_PRIMARY)
}

/// Card Title
///
/// The interface's center of gravity. Card titles, list rows, buttons, queue items. Most text lives here.
pub fn card_title(input: &str) -> Text<'_> {
    text(input).size(13).color(color::TEXT_SECONDARY)
}

/// Label
///
/// Design note: When inactive, labels should use [Variant::Alt].
///
/// The interface's center of gravity. Card titles, list rows, buttons, queue items. Most text lives here.
pub fn label(input: &str, ctx: Variant) -> Text<'_> {
    let color = match ctx {
        Variant::Main => color::TEXT_PRIMARY,
        Variant::Alt => color::TEXT_SECONDARY,
    };

    text(input).size(12).color(color)
}

/// Caption
///
/// The smallest text. Sub-captions, counts, badges and the all-caps eyebrows
/// that label every section.
pub fn caption(input: &str) -> Text<'_> {
    text(input)
        .font(font::regular())
        .size(11)
        .color(color::TEXT_SECONDARY)
}

/// Eyebrow
///
/// Design note: should be all caps ALWAYS.
///
/// The smallest text. Sub-captions, counts, badges and the all-caps eyebrows
/// that label every section.
pub fn eyebrow(input: &str, ctx: Variant) -> Text<'_> {
    let color = match ctx {
        Variant::Main => color::primary(),
        Variant::Alt => color::TEXT_SECONDARY,
    };

    let size = match ctx {
        Variant::Main => 10,
        Variant::Alt => 11,
    };

    let font = match ctx {
        Variant::Main => font::bold(),
        Variant::Alt => font::semibold(),
    };

    // TODO: Handle the letter spacing.
    text(input).font(font).size(size).color(color)
}

/// Lyric
///
/// Note: Main refers to the active lyric (full opacity) and alt is for queued entries.
pub fn lyric(input: &str, ctx: Variant) -> Text<'_> {
    let color = match ctx {
        Variant::Main => color::TEXT_PRIMARY,
        Variant::Alt => color::with_alpha(color::TEXT_PRIMARY, 0.5),
    };

    // TODO: Handle the letter spacing.
    text(input).font(font::semibold()).size(22).color(color)
}

/// Media Overlay
pub fn media_overlay(input: &str) -> Text<'_> {
    // TODO: Handle the letter spacing.
    text(input)
        .font(font::bold())
        .size(10)
        .color(color::with_alpha(color::TEXT_PRIMARY, 0.78))
}
