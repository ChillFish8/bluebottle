use iced::widget::{Column, Row, container, text_input};
use iced::{Center, Element, Length, Padding};

use super::focus_frame::{Shape, focus_frame, text_input_style};
use crate::widget::button::icon_flat;
use crate::widget::text::{self, Variant};
use crate::{color, icon, spacing};

const FIELD_HEIGHT: f32 = 46.0;
const EYE_DIAMETER: f32 = 34.0;
const EYE_GLYPH: f32 = 18.0;
/// Vertical padding inside the text_input so its hit bounds extend across the
/// full visual field height. Sized to centre size-14 / line-height-1.3 content.
const FIELD_PAD_Y: f32 = (FIELD_HEIGHT - 18.0) / 2.0;

enum Affix<Message> {
    None,
    Reveal { revealed: bool, on_toggle: Message },
}

/// A labelled rounded-rect text field in the bordered-glass family.
pub struct TextField<'a, Message> {
    label: &'a str,
    value: &'a str,
    placeholder: &'a str,
    optional: bool,
    help: Option<&'a str>,
    valid: bool,
    error: Option<&'a str>,
    disabled: bool,
    width: Length,
    on_input: Option<Box<dyn Fn(String) -> Message + 'a>>,
    on_submit: Option<Message>,
}

/// Build a [`TextField`].
pub fn text_field<'a, Message>(
    label: &'a str,
    value: &'a str,
) -> TextField<'a, Message> {
    TextField {
        label,
        value,
        placeholder: "",
        optional: false,
        help: None,
        valid: false,
        error: None,
        disabled: false,
        width: Length::Fill,
        on_input: None,
        on_submit: None,
    }
}

impl<'a, Message> TextField<'a, Message> {
    pub fn placeholder(mut self, placeholder: &'a str) -> Self {
        self.placeholder = placeholder;
        self
    }

    pub fn optional(mut self, on: bool) -> Self {
        self.optional = on;
        self
    }

    pub fn help(mut self, help: &'a str) -> Self {
        self.help = Some(help);
        self
    }

    pub fn valid(mut self, on: bool) -> Self {
        self.valid = on;
        self
    }

    pub fn error(mut self, message: &'a str) -> Self {
        self.error = Some(message);
        self
    }

    pub fn disabled(mut self, on: bool) -> Self {
        self.disabled = on;
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
}

/// A [`TextField`] with a trailing reveal eye and secured masking.
pub struct PasswordField<'a, Message> {
    inner: TextField<'a, Message>,
    revealed: bool,
    on_toggle_reveal: Option<Message>,
}

/// Build a [`PasswordField`].
pub fn password_field<'a, Message>(
    label: &'a str,
    value: &'a str,
    revealed: bool,
) -> PasswordField<'a, Message> {
    PasswordField {
        inner: text_field(label, value),
        revealed,
        on_toggle_reveal: None,
    }
}

impl<'a, Message> PasswordField<'a, Message> {
    pub fn placeholder(mut self, placeholder: &'a str) -> Self {
        self.inner = self.inner.placeholder(placeholder);
        self
    }

    pub fn optional(mut self, on: bool) -> Self {
        self.inner = self.inner.optional(on);
        self
    }

    pub fn help(mut self, help: &'a str) -> Self {
        self.inner = self.inner.help(help);
        self
    }

    pub fn valid(mut self, on: bool) -> Self {
        self.inner = self.inner.valid(on);
        self
    }

    pub fn error(mut self, message: &'a str) -> Self {
        self.inner = self.inner.error(message);
        self
    }

    pub fn disabled(mut self, on: bool) -> Self {
        self.inner = self.inner.disabled(on);
        self
    }

    pub fn width(mut self, width: impl Into<Length>) -> Self {
        self.inner = self.inner.width(width);
        self
    }

    pub fn on_input(mut self, on_input: impl Fn(String) -> Message + 'a) -> Self {
        self.inner = self.inner.on_input(on_input);
        self
    }

    pub fn on_submit(mut self, message: Message) -> Self {
        self.inner = self.inner.on_submit(message);
        self
    }

    pub fn on_toggle_reveal(mut self, message: Message) -> Self {
        self.on_toggle_reveal = Some(message);
        self
    }
}

fn render<'a, Message>(
    field: TextField<'a, Message>,
    affix: Affix<Message>,
    secure: bool,
) -> Element<'a, Message>
where
    Message: Clone + 'a,
{
    let label_variant = if field.disabled {
        Variant::Alt
    } else {
        Variant::Main
    };
    let label = text::label(field.label, label_variant);

    let header: Element<'a, Message> = if field.optional {
        Row::new()
            .spacing(spacing::GAP_8)
            .align_y(Center)
            .push(label)
            .push(text::caption("optional"))
            .into()
    } else {
        label.into()
    };

    // The text_input owns the horizontal padding so its hit bounds extend
    // across the field's visual inset. Padding on the focus_frame would leave
    // a dead zone where a click lights the ring without focusing the caret.
    let has_affix = matches!(affix, Affix::Reveal { .. });
    let input_right_pad = if has_affix { 0.0 } else { spacing::PAD_16 };

    let mut input = text_input(field.placeholder, field.value)
        .padding(Padding {
            top: FIELD_PAD_Y,
            right: input_right_pad,
            bottom: FIELD_PAD_Y,
            left: spacing::PAD_16,
        })
        .size(14)
        .width(Length::Fill)
        .secure(secure)
        .style(text_input_style);
    if !field.disabled {
        if let Some(on_input) = field.on_input {
            input = input.on_input(on_input);
        }
        if let Some(on_submit) = field.on_submit.clone() {
            input = input.on_submit(on_submit);
        }
    }

    let mut row = Row::new().spacing(0).align_y(Center).push(input);

    let frame_right_pad = match affix {
        Affix::None => 0.0,
        Affix::Reveal {
            revealed,
            on_toggle,
        } => {
            let glyph_name = if revealed {
                "visibility_off"
            } else {
                "visibility"
            };
            let message = if field.disabled {
                None
            } else {
                Some(on_toggle)
            };
            let eye =
                icon_flat(glyph_name, false, message).size(EYE_DIAMETER, EYE_GLYPH);
            row = row.push(eye);
            spacing::PAD_6
        },
    };

    let row = container(row)
        .height(Length::Fixed(FIELD_HEIGHT))
        .align_y(Center);

    let shell = focus_frame(row)
        .shape(Shape::Field)
        .error(field.error.is_some())
        .disabled(field.disabled)
        .padding(Padding {
            top: 0.0,
            right: frame_right_pad,
            bottom: 0.0,
            left: 0.0,
        })
        .height(Length::Fixed(FIELD_HEIGHT))
        .width(Length::Fill);

    let shell_el: Element<'a, Message> = shell.into();
    let mut column = Column::new()
        .spacing(spacing::GAP_6)
        .push(header)
        .push(shell_el);

    if let Some(error) = field.error {
        let glyph = icon::filled("error").size(14).color(color::error());
        let caption = text::caption(error).color(color::error());
        let row = Row::new()
            .spacing(spacing::GAP_6)
            .align_y(Center)
            .push(glyph)
            .push(caption);
        column = column.push(row);
    } else if field.valid {
        let glyph = icon::filled("check_circle")
            .size(14)
            .color(color::success());
        let caption =
            text::caption(field.help.unwrap_or("Looks good")).color(color::success());
        let row = Row::new()
            .spacing(spacing::GAP_6)
            .align_y(Center)
            .push(glyph)
            .push(caption);
        column = column.push(row);
    } else if let Some(help) = field.help {
        column = column.push(text::caption(help));
    }

    container(column)
        .width(field.width)
        .padding(Padding::ZERO)
        .into()
}

impl<'a, Message> From<TextField<'a, Message>> for Element<'a, Message>
where
    Message: Clone + 'a,
{
    fn from(field: TextField<'a, Message>) -> Self {
        render(field, Affix::None, false)
    }
}

impl<'a, Message> From<PasswordField<'a, Message>> for Element<'a, Message>
where
    Message: Clone + 'a,
{
    fn from(field: PasswordField<'a, Message>) -> Self {
        // Disabled forces secure on regardless of the reveal toggle. A field
        // that was revealed before being disabled must not leak the value
        // while the toggle is detached, since the user has no way to re-hide.
        let revealed = field.revealed && !field.inner.disabled;
        let affix = match field.on_toggle_reveal {
            Some(message) => Affix::Reveal {
                revealed,
                on_toggle: message,
            },
            None => Affix::None,
        };
        render(field.inner, affix, !revealed)
    }
}
