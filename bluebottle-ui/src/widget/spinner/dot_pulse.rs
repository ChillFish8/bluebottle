use std::f32::consts::PI;

use iced::advanced::widget::tree::{self, Tree};
use iced::advanced::{Clipboard, Layout, Shell, Widget, layout, renderer};
use iced::time::Instant;
use iced::widget::canvas;
use iced::{Element, Event, Length, Rectangle, Renderer, Size, Vector, mouse, window};

use super::bead::{Tone, fill, rim};

const BEAD_COUNT: usize = 3;
const WAVE_SECS: f32 = 1.1;
const STAGGER_SECS: f32 = 0.14;
const LOOP_SECS: f32 = WAVE_SECS + (BEAD_COUNT as f32 - 1.0) * STAGGER_SECS;
const REST_OPACITY: f32 = 0.3;

/// Bead diameter.
#[derive(Clone, Copy, Default, PartialEq, Eq, Debug)]
pub enum Diameter {
    Small,
    #[default]
    Medium,
}

impl Diameter {
    fn bead(self) -> f32 {
        match self {
            Self::Small => 8.0,
            Self::Medium => 11.0,
        }
    }

    fn rise(self) -> f32 {
        self.bead() * 0.4
    }

    fn gap(self) -> f32 {
        self.bead() * 0.4
    }
}

/// Three glass beads that rise and brighten in a staggered breath.
pub struct DotPulse {
    diameter: Diameter,
    tone: Tone,
}

/// Build a [`DotPulse`] with default size and tone.
pub fn dot_pulse() -> DotPulse {
    DotPulse {
        diameter: Diameter::default(),
        tone: Tone::default(),
    }
}

impl DotPulse {
    /// Sets the bead diameter.
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
    elapsed: f32,
    cache: canvas::Cache,
}

impl<Message, Theme> Widget<Message, Theme, Renderer> for DotPulse {
    fn size(&self) -> Size<Length> {
        let bead = self.diameter.bead();
        let gap = self.diameter.gap();
        let rise = self.diameter.rise();
        Size {
            width: Length::Fixed(
                BEAD_COUNT as f32 * bead + (BEAD_COUNT - 1) as f32 * gap,
            ),
            height: Length::Fixed(bead + rise),
        }
    }

    fn layout(
        &mut self,
        _tree: &mut Tree,
        _renderer: &Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        let Size { width, height } =
            <Self as Widget<Message, Theme, Renderer>>::size(self);
        layout::atomic(limits, width, height)
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
        let gap = self.diameter.gap();
        let rise = self.diameter.rise();
        let tone = self.tone;
        let elapsed = state.elapsed;

        let geometry = state.cache.draw(renderer, bounds.size(), |frame| {
            let bead_radius = bead / 2.0 - 0.5;

            for i in 0..BEAD_COUNT {
                let local = (elapsed - i as f32 * STAGGER_SECS).rem_euclid(LOOP_SECS);
                let amp = if local <= WAVE_SECS {
                    (local / WAVE_SECS * PI).sin().max(0.0)
                } else {
                    0.0
                };

                let opacity = REST_OPACITY + (1.0 - REST_OPACITY) * amp;

                let x = bead / 2.0 + i as f32 * (bead + gap);
                let y = bead / 2.0 + rise * (1.0 - amp);
                let path = canvas::Path::circle(iced::Point::new(x, y), bead_radius);

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
            state.elapsed = now.duration_since(start).as_secs_f32();
            state.cache.clear();
            shell.request_redraw();
        }
    }
}

impl<'a, Message, Theme> From<DotPulse> for Element<'a, Message, Theme, Renderer>
where
    Message: 'a,
    Theme: 'a,
{
    fn from(pulse: DotPulse) -> Self {
        Self::new(pulse)
    }
}
