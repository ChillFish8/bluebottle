use iced::widget::{Row, container, text_input};
use iced::{Center, Element, Length, Padding, Pixels};

use super::focus_frame::{Shape, focus_frame, text_input_style};
use crate::icon::{ICON_FILLED_FONT, filled_codepoint};
use crate::widget::button::icon_flat;

const ICON_GAP: f32 = 8.0;
const SHELL_LEFT_PAD: f32 = 12.0;
const SHELL_RIGHT_PAD: f32 = 6.0;
/// Content height for size-14 / line-height-1.3 text. The text_input's
/// vertical padding fills the pill height around this so the field's hit
/// bounds span the full visual height.
const CONTENT_HEIGHT: f32 = 18.0;

/// Component height.
#[derive(Clone, Copy, Default, Debug, Eq, PartialEq)]
pub enum SearchFieldSize {
    #[default]
    Standard,
    Dense,
}

struct Metrics {
    height: f32,
    magnifier: f32,
    clear_diameter: f32,
    clear_glyph: f32,
}

impl SearchFieldSize {
    fn metrics(self) -> Metrics {
        match self {
            Self::Standard => Metrics {
                height: 40.0,
                magnifier: 17.0,
                clear_diameter: 28.0,
                clear_glyph: 14.0,
            },
            Self::Dense => Metrics {
                height: 34.0,
                magnifier: 15.0,
                clear_diameter: 24.0,
                clear_glyph: 13.0,
            },
        }
    }
}

/// A focus-within search pill. Magnifier, transparent input, optional clear.
pub struct SearchField<'a, Message> {
    value: &'a str,
    placeholder: &'a str,
    size: SearchFieldSize,
    width: Length,
    on_input: Option<Box<dyn Fn(String) -> Message + 'a>>,
    on_submit: Option<Message>,
    on_clear: Option<Message>,
}

/// Build a [`SearchField`].
pub fn search_field<Message>(value: &str) -> SearchField<'_, Message> {
    SearchField {
        value,
        placeholder: "",
        size: SearchFieldSize::default(),
        width: Length::Fill,
        on_input: None,
        on_submit: None,
        on_clear: None,
    }
}

impl<'a, Message> SearchField<'a, Message> {
    pub fn placeholder(mut self, placeholder: &'a str) -> Self {
        self.placeholder = placeholder;
        self
    }

    pub fn size(mut self, size: SearchFieldSize) -> Self {
        self.size = size;
        self
    }

    pub fn width(mut self, width: impl Into<Length>) -> Self {
        self.width = width.into();
        self
    }

    pub fn on_input(mut self, on_input: impl Fn(String) -> Message + 'a) -> Self {
        self.on_input = Some(Box::new(on_input));
        self
    }

    pub fn on_submit(mut self, message: Message) -> Self {
        self.on_submit = Some(message);
        self
    }

    pub fn on_clear(mut self, message: Message) -> Self {
        self.on_clear = Some(message);
        self
    }
}

impl<'a, Message> From<SearchField<'a, Message>> for Element<'a, Message>
where
    Message: Clone + 'a,
{
    fn from(field: SearchField<'a, Message>) -> Self {
        let metrics = field.size.metrics();
        let has_clear = !field.value.is_empty() && field.on_clear.is_some();

        // The magnifier rides as the text_input's own leading icon so its
        // glyph area and the visual left inset are part of the input's hit
        // bounds. Otherwise a click on the magnifier would only light the
        // focus ring without actually focusing the caret.
        let magnifier_code = filled_codepoint("search")
            .chars()
            .next()
            .expect("search icon codepoint");
        let magnifier = text_input::Icon {
            font: ICON_FILLED_FONT,
            code_point: magnifier_code,
            size: Some(Pixels(metrics.magnifier)),
            spacing: ICON_GAP,
            side: text_input::Side::Left,
        };

        // The input owns its horizontal inset so the padded zone counts as
        // its hit area. The shell keeps only the right edge gap for the
        // optional clear button to breathe.
        let input_right_pad = if has_clear { 0.0 } else { SHELL_RIGHT_PAD };
        let pad_y = ((metrics.height - CONTENT_HEIGHT) / 2.0).max(0.0);
        let mut input = text_input(field.placeholder, field.value)
            .icon(magnifier)
            .padding(Padding {
                top: pad_y,
                right: input_right_pad,
                bottom: pad_y,
                left: SHELL_LEFT_PAD,
            })
            .size(14)
            .width(Length::Fill)
            .style(text_input_style);

        if let Some(on_input) = field.on_input {
            input = input.on_input(on_input);
        }

        if let Some(on_submit) = field.on_submit {
            input = input.on_submit(on_submit);
        }

        let mut row = Row::new().spacing(0).align_y(Center).push(input);

        let shell_right_pad = if has_clear {
            let on_clear = field.on_clear.expect("clear message");
            let clear = icon_flat("close", false, Some(on_clear))
                .size(metrics.clear_diameter, metrics.clear_glyph);
            row = row.push(clear);
            SHELL_RIGHT_PAD
        } else {
            0.0
        };

        let row = container(row)
            .height(Length::Fixed(metrics.height))
            .align_y(Center);

        focus_frame(row)
            .shape(Shape::Pill)
            .padding(Padding {
                top: 0.0,
                right: shell_right_pad,
                bottom: 0.0,
                left: 0.0,
            })
            .width(field.width)
            .height(Length::Fixed(metrics.height))
            .into()
    }
}
