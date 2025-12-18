use std::time::{Duration, Instant};

use iced::advanced::layout::{Limits, Node};
use iced::advanced::renderer::{Quad, Style};
use iced::advanced::widget::{Tree, tree};
use iced::advanced::{Clipboard, Layout, Shell, Widget, layout};
use iced::gradient::Linear;
use iced::{
    Background,
    Degrees,
    Element,
    Event,
    Gradient,
    Length,
    Rectangle,
    Size,
    advanced,
    border,
    mouse,
    window,
};

use crate::{color, easing};

/// A shimmer skeleton.
pub fn skeleton<'a>() -> Skeleton<'a> {
    Skeleton {
        width: Length::Fill,
        height: Length::Fill,
        border: border::Border::default().rounded(8),
        cycle_duration: Duration::from_millis(2000),
        easing: &easing::STANDARD,
    }
}

#[derive(Clone, Copy)]
struct State {
    progress: f32,
    direction: Direction,
    start: Instant,
}

impl Default for State {
    fn default() -> Self {
        Self {
            start: Instant::now(),
            direction: Direction::Forward,
            progress: 0.0,
        }
    }
}

impl State {
    fn timed_transition(&mut self, cycle_duration: Duration, now: Instant) {
        let elapsed = now.duration_since(self.start);

        match elapsed {
            elapsed if elapsed > cycle_duration => {
                self.progress = 0.0;
                self.start = now;
                self.direction.reverse();
            },
            _ => {
                self.progress = elapsed.as_secs_f32() / cycle_duration.as_secs_f32();
            },
        }
    }
}

/// A skeleton shimmer widget.
pub struct Skeleton<'a> {
    width: Length,
    height: Length,
    border: border::Border,
    cycle_duration: Duration,
    easing: &'a easing::Easing,
}

impl<'a> Skeleton<'a> {
    /// Sets the width of the [Skeleton].
    pub fn width(mut self, width: impl Into<Length>) -> Self {
        self.width = width.into();
        self
    }

    /// Sets the height of the [Skeleton].
    pub fn height(mut self, height: impl Into<Length>) -> Self {
        self.height = height.into();
        self
    }

    /// Sets the border of the [Skeleton].
    pub fn border(mut self, border: border::Border) -> Self {
        self.border = border;
        self
    }

    /// Sets the cycle duration of this [Skeleton].
    pub fn cycle_duration(mut self, duration: Duration) -> Self {
        self.cycle_duration = duration;
        self
    }

    /// Set the motion easing of this [Skeleton].
    pub fn easing(mut self, easing: &'a easing::Easing) -> Self {
        self.easing = easing;
        self
    }
}

impl<'a, Message, Theme, Renderer> Widget<Message, Theme, Renderer> for Skeleton<'a>
where
    Message: Clone + 'a,
    Renderer: advanced::Renderer + 'a,
{
    fn size(&self) -> Size<Length> {
        Size {
            width: self.width,
            height: self.height,
        }
    }

    fn layout(
        &mut self,
        _tree: &mut Tree,
        _renderer: &Renderer,
        limits: &Limits,
    ) -> Node {
        layout::atomic(limits, self.width, self.height)
    }

    fn draw(
        &self,
        tree: &Tree,
        renderer: &mut Renderer,
        _theme: &Theme,
        _style: &Style,
        layout: Layout<'_>,
        _cursor: mouse::Cursor,
        _viewport: &Rectangle,
    ) {
        let bounds = layout.bounds();
        let state = tree.state.downcast_ref::<State>();

        let offset = self.easing.y_at_x(state.progress);
        let rotation = match state.direction {
            Direction::Forward => 90.0,
            Direction::Backward => -90.0,
        };

        let linear = Linear::new(Degrees(rotation))
            .add_stop(0.0, color::SECONDARY)
            .add_stop(offset, color::SHIMMER)
            .add_stop(1.0, color::SECONDARY);

        renderer.fill_quad(
            Quad {
                bounds: Rectangle {
                    x: bounds.x,
                    y: bounds.y,
                    width: bounds.width,
                    height: bounds.height,
                },
                border: self.border,
                ..Quad::default()
            },
            Background::Gradient(Gradient::Linear(linear)),
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
        _renderer: &Renderer,
        _clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
        _viewport: &Rectangle,
    ) {
        let state = tree.state.downcast_mut::<State>();

        if let Event::Window(window::Event::RedrawRequested(now)) = event {
            state.timed_transition(self.cycle_duration, *now);
            shell.request_redraw();
        }
    }
}

impl<'a, Message> From<Skeleton<'a>> for Element<'a, Message>
where
    Message: Clone + 'a,
{
    fn from(value: Skeleton<'a>) -> Self {
        Self::new(value)
    }
}

#[derive(Copy, Clone, Eq, PartialEq)]
enum Direction {
    Forward,
    Backward,
}

impl Direction {
    fn reverse(&mut self) {
        *self = match *self {
            Direction::Forward => Direction::Backward,
            Direction::Backward => Direction::Forward,
        }
    }
}
