use iced::widget::{Row, container, text_input};
use iced::{Center, Element, Length, Padding};

use super::focus_frame::{Shape, focus_frame, text_input_style};
use crate::widget::button::icon_flat;
use crate::widget::text;
use crate::{color, font, spacing};

const FRAME_PAD: u16 = 5;

/// Component height.
#[derive(Clone, Copy, Default, Debug, Eq, PartialEq)]
pub enum StepperSize {
    #[default]
    Standard,
    Compact,
}

struct Metrics {
    button: f32,
    glyph: f32,
}

impl StepperSize {
    fn metrics(self) -> Metrics {
        match self {
            Self::Standard => Metrics {
                button: 36.0,
                glyph: 16.0,
            },
            Self::Compact => Metrics {
                button: 30.0,
                glyph: 14.0,
            },
        }
    }
}

/// A bounded numeric stepper.
pub struct Stepper<'a, Message> {
    value: i32,
    min: i32,
    max: i32,
    step: i32,
    size: StepperSize,
    on_change: Box<dyn Fn(i32) -> Message + 'a>,
    suffix: Option<&'a str>,
}

/// Build a [`Stepper`].
pub fn stepper<'a, Message>(
    value: i32,
    on_change: impl Fn(i32) -> Message + 'a,
) -> Stepper<'a, Message> {
    Stepper {
        value,
        min: i32::MIN,
        max: i32::MAX,
        step: 1,
        size: StepperSize::default(),
        on_change: Box::new(on_change),
        suffix: None,
    }
}

impl<'a, Message> Stepper<'a, Message> {
    pub fn min(mut self, min: i32) -> Self {
        self.min = min;
        self
    }

    pub fn max(mut self, max: i32) -> Self {
        self.max = max;
        self
    }

    pub fn step(mut self, step: i32) -> Self {
        self.step = step.max(1);
        self
    }

    pub fn size(mut self, size: StepperSize) -> Self {
        self.size = size;
        self
    }

    /// Adds a trailing static label after the editable value. The label is
    /// rendered as a separate widget so keystrokes still parse cleanly as
    /// integers. Use for unit suffixes like "%" or "px".
    pub fn suffix(mut self, suffix: &'a str) -> Self {
        self.suffix = Some(suffix);
        self
    }
}

fn measure_value_width(min: i32, max: i32) -> f32 {
    let candidates = [min.to_string(), max.to_string(), 0.to_string()];
    candidates
        .into_iter()
        .map(|s| {
            text::body(s)
                .font(font::mono_medium())
                .size(14)
                .shape_width()
        })
        .fold(0.0_f32, f32::max)
        .max(28.0)
}

impl<'a, Message> From<Stepper<'a, Message>> for Element<'a, Message>
where
    Message: Clone + 'a,
{
    fn from(stepper: Stepper<'a, Message>) -> Self {
        let metrics = stepper.size.metrics();
        let value = stepper.value.clamp(stepper.min, stepper.max);
        let at_min = value <= stepper.min;
        let at_max = value >= stepper.max;
        let on_change = stepper.on_change;

        let display = value.to_string();
        let display_width = measure_value_width(stepper.min, stepper.max);

        let minus_message = if at_min {
            None
        } else {
            Some((on_change.as_ref())(
                value.saturating_sub(stepper.step).max(stepper.min),
            ))
        };
        let minus = icon_flat("remove", false, minus_message)
            .size(metrics.button, metrics.glyph);

        let plus_message = if at_max {
            None
        } else {
            Some((on_change.as_ref())(
                value.saturating_add(stepper.step).min(stepper.max),
            ))
        };
        let plus =
            icon_flat("add", false, plus_message).size(metrics.button, metrics.glyph);

        let min = stepper.min;
        let max = stepper.max;
        let input = text_input(&display, &display)
            .padding(Padding::ZERO)
            .size(14)
            .font(font::mono_medium())
            .width(Length::Fixed(display_width))
            .align_x(iced::alignment::Horizontal::Center)
            .on_input(move |raw| {
                let trimmed = raw.trim();
                let parsed = if trimmed.is_empty() {
                    min
                } else {
                    trimmed.parse::<i32>().unwrap_or(value)
                };
                on_change(parsed.clamp(min, max))
            })
            .style(text_input_style);

        let value_block: Element<'a, Message> = match stepper.suffix {
            Some(suffix) => Row::new()
                .spacing(spacing::GAP_2)
                .align_y(Center)
                .push(input)
                .push(
                    text::body(suffix.to_string())
                        .font(font::mono_medium())
                        .size(14)
                        .color(color::TEXT_SECONDARY),
                )
                .into(),
            None => input.into(),
        };

        // Suffix decorations sit flush against the value block's right edge,
        // so without breathing room the plus button's hover veil reads as
        // bleeding into the suffix glyph. Bare numeric variants are centred
        // in their own budget and do not need the gap.
        let row_gap = if stepper.suffix.is_some() {
            spacing::GAP_4
        } else {
            0.0
        };

        let row = Row::new()
            .spacing(row_gap)
            .align_y(Center)
            .push(minus)
            .push(value_block)
            .push(plus);

        let row = container(row)
            .height(Length::Fixed(metrics.button + spacing::GAP_10))
            .align_y(Center);

        focus_frame(row)
            .shape(Shape::Pill)
            .padding(FRAME_PAD)
            .height(Length::Fixed(metrics.button + spacing::GAP_10))
            .into()
    }
}
