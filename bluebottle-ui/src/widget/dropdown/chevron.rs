//! A self-animating Material Icons chevron used by the dropdown chassis and
//! reusable by any variant that wants the same affordance.
//!
//! The glyph is `expand_more`. It rests pointing down and rotates 180 degrees
//! around its centre as `open` flips, on the design system's 100 ms hover
//! budget. The rotation goes through a canvas frame because iced's renderer
//! exposes only translate and scale on its public `Transformation`. Per
//! `use-icon-system-not-hand-drawn` the glyph stays sourced from the Material
//! Icons font, the canvas only carries the rotation.
//!
//! The chevron rides the cascaded text colour. Set [`Chevron::color`] to peg
//! it to a fixed tone, otherwise it picks up whatever the parent draws with.

use std::cell::Cell;
use std::time::Instant;

use iced::advanced::graphics::geometry::{
    Cache,
    Renderer as GeometryRenderer,
    Text as CanvasText,
};
use iced::advanced::widget::{Tree, tree};
use iced::advanced::{Clipboard, Layout, Renderer, Shell, Widget, layout, renderer};
use iced::widget::text::Alignment;
use iced::{
    Color,
    Element,
    Event,
    Length,
    Pixels,
    Radians,
    Rectangle,
    Size,
    Vector,
    alignment,
    mouse,
    window,
};

use crate::animate::hover::Hover;
use crate::{color, icon};

const DEFAULT_SIZE: f32 = 14.0;
const GLYPH: &str = "expand_more";

/// A chevron that points down when `open` is false and up when it is true.
pub fn chevron<Message>(open: bool) -> Chevron<Message> {
    Chevron {
        open,
        size: DEFAULT_SIZE,
        color: color::TEXT_SECONDARY,
        _marker: std::marker::PhantomData,
    }
}

/// A 14 px Material Icons chevron.
pub struct Chevron<Message> {
    open: bool,
    size: f32,
    color: Color,
    _marker: std::marker::PhantomData<fn() -> Message>,
}

impl<Message> Chevron<Message> {
    /// Sets the glyph size. Defaults to 14 px.
    pub fn size(mut self, size: f32) -> Self {
        self.size = size;
        self
    }

    /// Pegs the glyph to a fixed colour instead of riding the cascade.
    pub fn color(mut self, color: Color) -> Self {
        self.color = color;
        self
    }
}

struct State {
    rotation: Hover,

    /// Reuses the rasterised glyph between frames. `draw` re-rasterises when
    /// the live rotation diverges from the cached factor so a settled frame
    /// lands on the exact target angle.
    cache: Cache<iced::Renderer>,

    /// Rotation factor the cache was last populated with. `None` forces the
    /// next draw to repopulate.
    last_factor: Cell<Option<f32>>,

    /// Animating-state observed at the most recent redraw. Holds across the
    /// animating-to-settled edge so one extra redraw fires and the cache
    /// repopulates at the target angle.
    was_animating: Cell<bool>,
}

impl Default for State {
    fn default() -> Self {
        Self {
            rotation: Hover::default(),
            cache: Cache::new(),
            last_factor: Cell::new(None),
            was_animating: Cell::new(false),
        }
    }
}

impl<Message> Widget<Message, iced::Theme, iced::Renderer> for Chevron<Message> {
    fn size(&self) -> Size<Length> {
        Size {
            width: Length::Fixed(self.size),
            height: Length::Fixed(self.size),
        }
    }

    fn layout(
        &mut self,
        _tree: &mut Tree,
        _renderer: &iced::Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        let bounded = limits
            .width(Length::Fixed(self.size))
            .height(Length::Fixed(self.size))
            .resolve(
                Length::Fixed(self.size),
                Length::Fixed(self.size),
                Size::new(self.size, self.size),
            );
        layout::Node::new(bounded)
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
        let factor = state.rotation.current(Instant::now());
        let bounds = layout.bounds();

        let stale = match state.last_factor.get() {
            Some(last) => (factor - last).abs() > f32::EPSILON,
            None => true,
        };

        if stale {
            state.cache.clear();
            state.last_factor.set(Some(factor));
        }

        let text_color = self.color;
        let glyph_size = self.size;

        let geometry = state.cache.draw(renderer, bounds.size(), |frame| {
            let center = frame.center();

            frame.translate(Vector::new(center.x, center.y));
            frame.rotate(Radians(factor * std::f32::consts::PI));
            frame.translate(Vector::new(-center.x, -center.y));

            frame.fill_text(CanvasText {
                content: icon::filled_codepoint(GLYPH).to_string(),
                position: center,
                color: text_color,
                size: Pixels(glyph_size),
                font: icon::ICON_FILLED_FONT,
                align_x: Alignment::Center,
                align_y: alignment::Vertical::Center,
                ..CanvasText::default()
            });
        });

        renderer.with_translation(Vector::new(bounds.x, bounds.y), |renderer| {
            renderer.draw_geometry(geometry);
        });
    }

    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<State>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(State {
            rotation: Hover::settled(self.open),
            cache: Cache::new(),
            last_factor: Cell::new(None),
            was_animating: Cell::new(false),
        })
    }

    fn diff(&self, tree: &mut Tree) {
        let state = tree.state.downcast_mut::<State>();

        if state.rotation.flip(self.open, Instant::now()) {
            // A flip mid-flight retargets at the same instantaneous factor
            // that is already cached. Drop the cache so the next draw lays
            // down the new curve.
            state.last_factor.set(None);
        }
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
        // Gate on RedrawRequested. A non-redraw event arriving between the
        // settling tick and the next frame would otherwise clobber
        // `was_animating` and the post-settle redraw would never fire.
        let Event::Window(window::Event::RedrawRequested(_)) = event else {
            return;
        };

        let state = tree.state.downcast_ref::<State>();
        let animating = state.rotation.animating(Instant::now());

        // One extra redraw on the animating-to-settled edge lets `draw`
        // repopulate the cache at the exact target angle.
        if animating || state.was_animating.get() {
            shell.request_redraw();
        }

        state.was_animating.set(animating);
    }
}

impl<'a, Message: 'a> From<Chevron<Message>> for Element<'a, Message> {
    fn from(c: Chevron<Message>) -> Self {
        Element::new(c)
    }
}
