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
use crate::icon;

const DEFAULT_SIZE: f32 = 14.0;
const GLYPH: &str = "expand_more";

/// A chevron that points down when `open` is false and up when it is true.
pub fn chevron<Message>(open: bool) -> Chevron<Message> {
    Chevron {
        open,
        size: DEFAULT_SIZE,
        color: None,
        _marker: std::marker::PhantomData,
    }
}

/// A 14 px Material Icons chevron.
pub struct Chevron<Message> {
    open: bool,
    size: f32,
    color: Option<Color>,
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
        self.color = Some(color);
        self
    }
}

struct State {
    rotation: Hover,
    /// Reuses the rasterised glyph between settled frames. Cleared every event
    /// while the rotation is in flight; the resting and fully-open states then
    /// share the cached geometry until the next flip.
    cache: Cache<iced::Renderer>,
}

impl Default for State {
    fn default() -> Self {
        Self {
            rotation: Hover::default(),
            cache: Cache::new(),
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
        style: &renderer::Style,
        layout: Layout<'_>,
        _cursor: mouse::Cursor,
        _viewport: &Rectangle,
    ) {
        let state = tree.state.downcast_ref::<State>();
        let factor = state.rotation.current(Instant::now());
        let bounds = layout.bounds();
        let text_color = self.color.unwrap_or(style.text_color);
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
        })
    }

    fn diff(&self, tree: &mut Tree) {
        let state = tree.state.downcast_mut::<State>();
        if state.rotation.flip(self.open, Instant::now()) {
            // A new tween has started; previous-frame glyph is stale.
            state.cache.clear();
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
        let state = tree.state.downcast_ref::<State>();
        let now = Instant::now();
        if state.rotation.animating(now) {
            // Mid-tween the rotation moves every frame; invalidate so the next
            // draw re-rasterises at the current angle. When settled the cache
            // sticks and the closure is skipped.
            state.cache.clear();
            if let Event::Window(window::Event::RedrawRequested(_)) = event {
                shell.request_redraw();
            }
        }
    }
}

impl<'a, Message: 'a> From<Chevron<Message>> for Element<'a, Message> {
    fn from(c: Chevron<Message>) -> Self {
        Element::new(c)
    }
}
