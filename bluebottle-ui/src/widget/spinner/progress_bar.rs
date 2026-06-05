use std::time::Duration;

use iced::advanced::renderer::{self, Quad};
use iced::advanced::widget::tree::{self, Tree};
use iced::advanced::{Clipboard, Layout, Renderer, Shell, Widget, layout};
use iced::time::Instant;
use iced::widget::Row;
use iced::{Background, Element, Event, Length, Rectangle, Size, mouse, window};

use super::bead::Tone;
use crate::border::Radius;
use crate::widget::text;
use crate::{color, easing, font, style};

const TRACK_HEIGHT: f32 = 8.0;
const INDETERMINATE_FRACTION: f32 = 0.4;
const INDETERMINATE_CYCLE: Duration = Duration::from_millis(1400);
const FILL_ALPHA: f32 = 0.62;
const LABEL_SPACING: f32 = 12.0;

/// A glass-rail progress indicator. Indeterminate by default.
pub struct ProgressBar {
    width: Length,
    value: Option<f32>,
    tone: Tone,
}

/// Build a [`ProgressBar`].
pub fn progress_bar() -> ProgressBar {
    ProgressBar {
        width: Length::Fill,
        value: None,
        tone: Tone::default(),
    }
}

impl ProgressBar {
    /// Sets the total component width. In the determinate variant the read-out
    /// label takes its natural width from this budget and the rail fills the
    /// rest.
    pub fn width(mut self, width: impl Into<Length>) -> Self {
        self.width = width.into();
        self
    }

    /// Sets the bar to determinate at `value` in `[0, 1]`.
    pub fn value(mut self, value: f32) -> Self {
        self.value = Some(value.clamp(0.0, 1.0));
        self
    }

    /// Sets the fill tone.
    pub fn tone(mut self, tone: Tone) -> Self {
        self.tone = tone;
        self
    }
}

#[derive(Default)]
struct State {
    start: Option<Instant>,
    phase: f32,
}

struct Rail {
    width: Length,
    value: Option<f32>,
    tone: Tone,
}

impl Rail {
    fn fill_color(&self) -> iced::Color {
        let base = match self.tone {
            Tone::Accent => color::primary(),
            Tone::White => color::WHITE,
        };
        color::with_alpha(base, color::srgb_alpha(FILL_ALPHA))
    }
}

impl<Message> Widget<Message, iced::Theme, iced::Renderer> for Rail {
    fn size(&self) -> Size<Length> {
        Size {
            width: self.width,
            height: Length::Fixed(TRACK_HEIGHT),
        }
    }

    fn layout(
        &mut self,
        _tree: &mut Tree,
        _renderer: &iced::Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        layout::atomic(limits, self.width, Length::Fixed(TRACK_HEIGHT))
    }

    fn draw(
        &self,
        tree: &Tree,
        renderer: &mut iced::Renderer,
        _theme: &iced::Theme,
        _style: &renderer::Style,
        layout: Layout<'_>,
        _cursor: mouse::Cursor,
        _viewport: &Rectangle,
    ) {
        let bounds = layout.bounds();
        let radius = Radius::new(bounds.height / 2.0);

        renderer.fill_quad(
            Quad {
                bounds,
                border: style::hairline(color::border_strong()).rounded(radius),
                ..Quad::default()
            },
            Background::Color(color::hover_veil()),
        );

        let fill = match self.value {
            Some(value) => Rectangle {
                x: bounds.x,
                y: bounds.y,
                width: bounds.width * value,
                height: bounds.height,
            },
            None => {
                let state = tree.state.downcast_ref::<State>();
                let segment = bounds.width * INDETERMINATE_FRACTION;
                let travel = bounds.width + segment;
                let eased = easing::EMPHASIZED_ACCELERATE.y_at_x(state.phase);
                let raw_x = bounds.x - segment + travel * eased;

                let left = raw_x.max(bounds.x);
                let right = (raw_x + segment).min(bounds.x + bounds.width);

                Rectangle {
                    x: left,
                    y: bounds.y,
                    width: (right - left).max(0.0),
                    height: bounds.height,
                }
            },
        };

        if fill.width <= 0.0 {
            return;
        }

        let rim = match self.tone {
            Tone::Accent => color::primary(),
            Tone::White => color::border_strong(),
        };

        renderer.fill_quad(
            Quad {
                bounds: fill,
                border: style::hairline(rim).rounded(radius),
                ..Quad::default()
            },
            Background::Color(self.fill_color()),
        );
    }

    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<State>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(State::default())
    }

    fn update(
        &mut self,
        tree: &mut Tree,
        event: &Event,
        _layout: Layout<'_>,
        _cursor: mouse::Cursor,
        _renderer: &iced::Renderer,
        _clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
        _viewport: &Rectangle,
    ) {
        if self.value.is_some() {
            return;
        }

        let state = tree.state.downcast_mut::<State>();

        if let Event::Window(window::Event::RedrawRequested(now)) = event {
            let start = *state.start.get_or_insert(*now);
            let cycle = INDETERMINATE_CYCLE.as_secs_f32();
            let elapsed = now.duration_since(start).as_secs_f32();
            state.phase = (elapsed / cycle).fract();
            shell.request_redraw();
        }
    }
}

fn read_out(value: f32) -> text::Text<'static> {
    let percent = (value * 100.0).round() as i32;
    text::caption(format!("{percent}%"))
        .font(font::mono_medium())
        .size(13)
        .color(color::TEXT_MUTED)
}

impl<'a, Message> From<ProgressBar> for Element<'a, Message>
where
    Message: 'a,
{
    fn from(bar: ProgressBar) -> Self {
        match bar.value {
            None => Element::new(Rail {
                width: bar.width,
                value: None,
                tone: bar.tone,
            }),
            Some(value) => {
                let rail: Element<'a, Message> = Element::new(Rail {
                    width: Length::Fill,
                    value: Some(value),
                    tone: bar.tone,
                });
                let label: Element<'a, Message> = read_out(value).into();

                Row::with_children([rail, label])
                    .width(bar.width)
                    .align_y(iced::Center)
                    .spacing(LABEL_SPACING)
                    .into()
            },
        }
    }
}
