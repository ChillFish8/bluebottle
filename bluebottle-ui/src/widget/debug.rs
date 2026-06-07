use iced::advanced::renderer::{self, Quad};
use iced::advanced::text::{self, Renderer as TextRenderer};
use iced::advanced::widget::tree::{self, Tree};
use iced::advanced::{Clipboard, Layout, Renderer, Shell, Widget, layout};
use iced::time::Instant;
use iced::{
    Background,
    Border,
    Element,
    Event,
    Length,
    Pixels,
    Point,
    Rectangle,
    Size,
    alignment,
    border,
    mouse,
    widget,
    window,
};

use crate::{color, font};

/// Wrap the element in a container with debug box lines.
pub fn container<'a, Message>(
    element: impl Into<Element<'a, Message>>,
) -> Element<'a, Message>
where
    Message: 'a,
{
    widget::container(element)
        .style(|_theme| widget::container::Style {
            text_color: None,
            background: None,
            border: border::Border::default().width(1).color(color::primary()),
            shadow: Default::default(),
            snap: true,
        })
        .into()
}

/// A floating pill that reads out the achieved render rate in frames per second.
///
/// It measures the gap between successive redraws and keeps requesting more, so
/// it settles to whatever rate the runtime actually presents. Drop it into a
/// corner with a stack to keep an eye on frame pacing while developing.
pub fn fps_counter() -> FpsCounter {
    FpsCounter
}

/// The fixed pill dimensions. Wide enough for a three digit reading.
const SIZE: Size = Size::new(64.0, 26.0);

/// Weight given to the newest sample when smoothing the rate. Lower is steadier.
const SMOOTHING: f32 = 0.1;

/// Shortest gap that counts as a real frame. A single present can deliver
/// several redraw ticks microseconds apart while the interface settles, so a
/// gap shorter than this is treated as a duplicate and skipped. This caps the
/// reading near 500fps, which is plenty for a debug overlay.
const MIN_SAMPLE_SECS: f32 = 0.002;

/// See [`fps_counter`].
pub struct FpsCounter;

#[derive(Default)]
struct State {
    last: Option<Instant>,
    smoothed: f32,
    displayed: u32,
}

impl<Message> Widget<Message, iced::Theme, iced::Renderer> for FpsCounter {
    fn size(&self) -> Size<Length> {
        Size {
            width: Length::Fixed(SIZE.width),
            height: Length::Fixed(SIZE.height),
        }
    }

    fn layout(
        &mut self,
        _tree: &mut Tree,
        _renderer: &iced::Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        layout::atomic(
            limits,
            Length::Fixed(SIZE.width),
            Length::Fixed(SIZE.height),
        )
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
        let state = tree.state.downcast_ref::<State>();
        let bounds = layout.bounds();

        renderer.fill_quad(
            Quad {
                bounds,
                border: Border::default()
                    .rounded(8)
                    .width(1)
                    .color(color::border_strong()),
                ..Quad::default()
            },
            Background::Color(color::with_alpha(color::SECONDARY, 0.85)),
        );

        let label = text::Text {
            content: format!("{} fps", state.displayed),
            bounds: bounds.size(),
            size: Pixels(12.0),
            line_height: text::LineHeight::default(),
            font: font::semibold(),
            align_x: text::Alignment::Center,
            align_y: alignment::Vertical::Center,
            shaping: text::Shaping::Basic,
            wrapping: text::Wrapping::None,
        };

        renderer.fill_text(
            label,
            Point::new(bounds.center_x(), bounds.center_y()),
            color::TEXT_PRIMARY,
            bounds,
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
        let state = tree.state.downcast_mut::<State>();

        if let Event::Window(window::Event::RedrawRequested(now)) = event {
            if let Some(last) = state.last {
                let delta = now.duration_since(last).as_secs_f32();
                if delta >= MIN_SAMPLE_SECS {
                    let instant = 1.0 / delta;
                    state.smoothed = if state.smoothed == 0.0 {
                        instant
                    } else {
                        state.smoothed * (1.0 - SMOOTHING) + instant * SMOOTHING
                    };
                    state.displayed = state.smoothed.round() as u32;
                }
            }
            state.last = Some(*now);

            // Keep the frames coming so the reading stays live.
            shell.request_redraw();
        }
    }
}

impl<Message> From<FpsCounter> for Element<'_, Message> {
    fn from(counter: FpsCounter) -> Self {
        Element::new(counter)
    }
}
