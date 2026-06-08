//! Visual rules and markers that divide content.
//!
//! Five forms. A [section_rule] introduces a block with a labelled, left-anchored
//! fade. A [terminal_marker] closes a finite list with mirrored fades around a
//! centered label. A [solid_rule] divides groups inside menus where an enclosing
//! border earns it the right to be solid. A [structural_border] anchors sticky
//! chrome and eases in on scroll. An [inline_dot] separates run-on metadata
//! within a single line.

use std::f32::consts::FRAC_PI_2;

use iced::gradient::Linear;
use iced::widget::{Container, Row, container, row, space};
use iced::{
    Background,
    Border,
    Center,
    Color,
    Element,
    Length,
    Radians,
    Theme,
    padding,
};

use crate::widget::text::{self as ui_text, Variant};
use crate::{color, font, spacing};

/// Hairline thickness in logical pixels.
const RULE_THICKNESS: f32 = 1.0;

/// Inline metadata dot diameter in logical pixels.
const DOT_SIZE: f32 = 2.0;

/// A caller-supplied label followed by a hairline that fades to nothing at the
/// far edge. The default block introducer. Pair with [section_label] for the
/// stock eyebrow styling, or pass any element to mix in a count, an icon, or a
/// link.
pub fn section_rule<'a, Message>(
    label: impl Into<Element<'a, Message>>,
) -> Row<'a, Message>
where
    Message: 'a,
{
    row![
        label.into(),
        fade_rule(color::border(), color::with_alpha(color::border(), 0.0))
    ]
    .align_y(Center)
    .spacing(spacing::GAP_4)
}

/// Stock label styling for [section_rule]. Uppercases the input and applies the
/// 10px bold eyebrow at the muted text colour.
pub fn section_label(text: &str) -> ui_text::Text<'_> {
    ui_text::eyebrow(text.to_uppercase(), Variant::Main)
        .color(color::TEXT_SECONDARY)
        .letter_spacing(1.2)
}

/// Two mirrored fades converge on a centered caps label. Reserved for true
/// ends. END OF SERIES, the bottom of a finished queue.
pub fn terminal_marker<'a, Message>(label: &str) -> Row<'a, Message>
where
    Message: 'a,
{
    let label = ui_text::eyebrow(label.to_uppercase(), Variant::Main)
        .font(font::semibold())
        .letter_spacing(1.2)
        .color(color::TEXT_DARK);

    let transparent = color::with_alpha(color::border(), 0.0);

    row![
        fade_rule(transparent, color::border()),
        label,
        fade_rule(color::border(), transparent),
    ]
    .align_y(Center)
    .spacing(spacing::GAP_12)
}

/// A full-strength hairline inset inside an enclosed surface. Use for menus
/// and dropdowns where the parent border carries the structural weight.
pub fn solid_rule<'a, Message>() -> Container<'a, Message>
where
    Message: 'a,
{
    let line = container(space().width(Length::Fill).height(RULE_THICKNESS)).style(
        |_theme: &Theme| container::Style {
            background: Some(Background::Color(color::border_strong())),
            ..container::Style::default()
        },
    );

    container(line).padding(padding::Padding {
        top: spacing::PAD_6,
        right: spacing::PAD_4,
        bottom: spacing::PAD_6,
        left: spacing::PAD_4,
    })
}

/// A 1px building edge under sticky chrome. `opacity` runs from 0 at rest to 1
/// once content has scrolled beneath it.
pub fn structural_border<'a, Message>(opacity: f32) -> Container<'a, Message>
where
    Message: 'a,
{
    let tint = color::fade(color::border(), opacity.clamp(0.0, 1.0));

    container(space().width(Length::Fill).height(RULE_THICKNESS)).style(
        move |_theme: &Theme| container::Style {
            background: Some(Background::Color(tint)),
            ..container::Style::default()
        },
    )
}

/// A 2px round dot separating run-on metadata. The smallest separator in the
/// system. It divides words, not regions.
pub fn inline_dot<'a, Message>() -> Container<'a, Message>
where
    Message: 'a,
{
    let dot =
        container(space().width(DOT_SIZE).height(DOT_SIZE)).style(|_theme: &Theme| {
            container::Style {
                background: Some(Background::Color(color::TEXT_DARK)),
                border: Border::default().rounded(DOT_SIZE / 2.0),
                ..container::Style::default()
            }
        });

    container(dot).padding(padding::Padding {
        top: 0.0,
        right: spacing::GAP_8,
        bottom: 0.0,
        left: spacing::GAP_8,
    })
}

/// A 1px hairline that linearly interpolates from `start` on the left edge to
/// `end` on the right. Fills the available width.
fn fade_rule<'a, Message>(start: Color, end: Color) -> Container<'a, Message>
where
    Message: 'a,
{
    container(space().width(Length::Fill).height(RULE_THICKNESS)).style(
        move |_theme: &Theme| {
            let gradient = Linear::new(Radians(FRAC_PI_2))
                .add_stop(0.0, start)
                .add_stop(1.0, end);

            container::Style {
                background: Some(Background::Gradient(gradient.into())),
                ..container::Style::default()
            }
        },
    )
}
