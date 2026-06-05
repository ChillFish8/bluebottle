use std::f32::consts::PI;
use std::time::Duration;

use iced::advanced::widget::tree::{self, Tree};
use iced::advanced::{Clipboard, Layout, Shell, Widget, layout, renderer};
use iced::time::Instant;
use iced::widget::canvas;
use iced::{Element, Event, Length, Rectangle, Renderer, Size, Vector, mouse, window};

use super::bead::{Tone, fill, rim};

const BEAD_COUNT: usize = 8;
const CYCLE: Duration = Duration::from_millis(1200);
const PEAK: f32 = 1.0 / BEAD_COUNT as f32;

/// Container diameter for the ring.
#[derive(Clone, Copy, Default, PartialEq, Eq, Debug)]
pub enum Diameter {
    Small,
    #[default]
    Medium,
    Large,
}

impl Diameter {
    fn ring(self) -> f32 {
        match self {
            Self::Small => 24.0,
            Self::Medium => 36.0,
            Self::Large => 48.0,
        }
    }

    fn bead(self) -> f32 {
        match self {
            Self::Small => 4.0,
            Self::Medium => 6.0,
            Self::Large => 7.0,
        }
    }
}

/// A ring of stationary glass beads whose opacities cycle in sequence to
/// read as a rotating bright spot trailing into the dark.
pub struct DotRing {
    diameter: Diameter,
    tone: Tone,
}

/// Build a [`DotRing`] with default size and tone.
pub fn dot_ring() -> DotRing {
    DotRing {
        diameter: Diameter::default(),
        tone: Tone::default(),
    }
}

impl DotRing {
    /// Sets the container diameter.
    pub fn diameter(mut self, diameter: Diameter) -> Self {
        self.diameter = diameter;
        self
    }

    /// Sets the bead tone.
    pub fn tone(mut self, tone: Tone) -> Self {
        self.tone = tone;
        self
    }
}

#[derive(Default)]
struct State {
    start: Option<Instant>,
    phase: f32,
    cache: canvas::Cache,
}

impl<Message, Theme> Widget<Message, Theme, Renderer> for DotRing {
    fn size(&self) -> Size<Length> {
        let side = Length::Fixed(self.diameter.ring());
        Size {
            width: side,
            height: side,
        }
    }

    fn layout(
        &mut self,
        _tree: &mut Tree,
        _renderer: &Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        let side = Length::Fixed(self.diameter.ring());
        layout::atomic(limits, side, side)
    }

    fn draw(
        &self,
        tree: &Tree,
        renderer: &mut Renderer,
        _theme: &Theme,
        _style: &renderer::Style,
        layout: Layout<'_>,
        _cursor: mouse::Cursor,
        _viewport: &Rectangle,
    ) {
        use iced::advanced::Renderer as _;

        let state = tree.state.downcast_ref::<State>();
        let bounds = layout.bounds();
        let bead = self.diameter.bead();
        let tone = self.tone;
        let phase = state.phase;

        let geometry = state.cache.draw(renderer, bounds.size(), |frame| {
            let centre = frame.center();
            let ring_radius = (frame.width() - bead) / 2.0;
            let bead_radius = bead / 2.0 - 0.5;
            let step = 2.0 * PI / BEAD_COUNT as f32;

            for i in 0..BEAD_COUNT {
                let angle = i as f32 * step;
                let bead_phase = (phase - i as f32 / BEAD_COUNT as f32).rem_euclid(1.0);
                let opacity = if bead_phase < PEAK {
                    bead_phase / PEAK
                } else {
                    let t = (bead_phase - PEAK) / (1.0 - PEAK);
                    (1.0 - t).powi(2)
                };

                let position = iced::Point::new(
                    centre.x + angle.cos() * ring_radius,
                    centre.y + angle.sin() * ring_radius,
                );
                let path = canvas::Path::circle(position, bead_radius);

                frame.fill(&path, crate::color::fade(fill(tone), opacity));
                frame.stroke(
                    &path,
                    canvas::Stroke::default()
                        .with_color(crate::color::fade(rim(tone), opacity))
                        .with_width(1.0),
                );
            }
        });

        renderer.with_translation(Vector::new(bounds.x, bounds.y), |renderer| {
            use iced::advanced::graphics::geometry::Renderer as _;

            renderer.draw_geometry(geometry);
        });
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
            let start = *state.start.get_or_insert(*now);
            let cycle = CYCLE.as_secs_f32();
            let elapsed = now.duration_since(start).as_secs_f32();
            state.phase = (elapsed / cycle).fract();

            state.cache.clear();
            shell.request_redraw();
        }
    }
}

impl<'a, Message, Theme> From<DotRing> for Element<'a, Message, Theme, Renderer>
where
    Message: 'a,
    Theme: 'a,
{
    fn from(ring: DotRing) -> Self {
        Self::new(ring)
    }
}
