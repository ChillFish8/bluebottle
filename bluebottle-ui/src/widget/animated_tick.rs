//! A self-animating check glyph.
//!
//! On the rising edge of `selected` the check strokes in as a single
//! continuous path. The pen starts at the upper-left tip, sweeps down to the
//! joint, and continues up to the upper-right tip, reading as a handwritten
//! check mark. While still selected the check sits at full opacity. On the
//! falling edge the fully drawn check fades out on the design system's hover
//! budget. Re-selection always restarts the draw-in, never snaps to
//! fully-drawn.
//!
//! Used by the dropdown menu rows and the bordered glass checkbox button.
//! The partial-stroke reveal shares the helpers in
//! [`crate::widget::path_trace`] with the animated nav puck border.

use std::cell::Cell;
use std::time::Instant;

use iced::advanced::graphics::geometry::{Cache, Renderer as GeometryRenderer};
use iced::advanced::widget::{Tree, tree};
use iced::advanced::{Clipboard, Layout, Renderer, Shell, Widget, layout, renderer};
use iced::widget::canvas;
use iced::{
    Color,
    Element,
    Event,
    Length,
    Point,
    Rectangle,
    Size,
    Vector,
    mouse,
    window,
};

use crate::animate::hover::{EPSILON, Hover};
use crate::widget::path_trace::trace_partial;
use crate::{color, style};

/// Stroke width per pixel of glyph size. At the default 14 px tick this lands
/// around 1.75 px, close to the Material `check` glyph weight.
const STROKE_RATIO: f32 = 0.125;

/// Proportions of the three check vertices in a unit square. Match the
/// Material `check` glyph so a settled tick reads the same as the icon-font
/// version did.
const P_LEFT: (f32, f32) = (0.18, 0.53);
const P_JOINT: (f32, f32) = (0.38, 0.73);
const P_RIGHT: (f32, f32) = (0.82, 0.29);

/// Builds an animated tick at the given glyph size. The check strokes in
/// while `selected` is true and fades out while it is false. Picks up the
/// cascaded `text_color` from the renderer style by default. Override with
/// [`AnimatedTick::color`].
pub fn animated_tick<'a, Message: 'a>(
    selected: bool,
    size: f32,
) -> AnimatedTick<'a, Message> {
    AnimatedTick {
        selected,
        size,
        color: None,
        _marker: std::marker::PhantomData,
    }
}

/// The widget returned by [`animated_tick`].
pub struct AnimatedTick<'a, Message> {
    selected: bool,
    size: f32,
    color: Option<Color>,
    _marker: std::marker::PhantomData<&'a fn() -> Message>,
}

impl<Message> AnimatedTick<'_, Message> {
    /// Overrides the stroke colour. Defaults to the cascaded `text_color`
    /// from the renderer style so the tick eases with its parent clickable.
    pub fn color(mut self, color: Color) -> Self {
        self.color = Some(color);
        self
    }
}

struct State {
    stroke: Hover,
    alpha: Hover,
    last_selected: bool,

    cache: Cache<iced::Renderer>,
    last_factor: Cell<Option<f32>>,
    last_alpha: Cell<Option<f32>>,
    last_primary: Cell<Option<Color>>,
    was_animating: Cell<bool>,
}

impl<'a, Message> Widget<Message, iced::Theme, iced::Renderer>
    for AnimatedTick<'a, Message>
{
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
        let now = Instant::now();
        let factor = state.stroke.current(now);
        let alpha = state.alpha.current(now);

        if alpha < EPSILON || factor < EPSILON {
            return;
        }

        // Default to the cascaded text colour so the tick rides whatever
        // colour a parent clickable is easing through. `color()` overrides
        // pin a specific tint for callers that need to ignore the cascade.
        let stroke = self.color.unwrap_or(style.text_color);
        let stale = match (
            state.last_factor.get(),
            state.last_alpha.get(),
            state.last_primary.get(),
        ) {
            (Some(lf), Some(la), Some(lp)) => {
                (factor - lf).abs() > f32::EPSILON
                    || (alpha - la).abs() > f32::EPSILON
                    || lp != stroke
            },
            _ => true,
        };

        if stale {
            state.cache.clear();
            state.last_factor.set(Some(factor));
            state.last_alpha.set(Some(alpha));
            state.last_primary.set(Some(stroke));
        }

        let bounds = layout.bounds();
        let size = self.size;
        let stroke_width = size * STROKE_RATIO;
        let stroke_color = color::with_alpha(stroke, alpha);

        let geometry = state.cache.draw(renderer, bounds.size(), |frame| {
            let scale = |(x, y): (f32, f32)| Point::new(x * size, y * size);
            let route = [scale(P_LEFT), scale(P_JOINT), scale(P_RIGHT)];

            let mut builder = canvas::path::Builder::new();
            trace_partial(&mut builder, &route, factor);

            frame.stroke(
                &builder.build(),
                canvas::Stroke::default()
                    .with_color(stroke_color)
                    .with_width(stroke_width)
                    .with_line_cap(canvas::LineCap::Round)
                    .with_line_join(canvas::LineJoin::Round),
            );
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
            stroke: Hover::settled(self.selected).with_fade(style::EMPHASIS),
            alpha: Hover::settled(self.selected),
            last_selected: self.selected,
            cache: Cache::new(),
            last_factor: Cell::new(None),
            last_alpha: Cell::new(None),
            last_primary: Cell::new(None),
            was_animating: Cell::new(false),
        })
    }

    fn diff(&self, tree: &mut Tree) {
        let state = tree.state.downcast_mut::<State>();

        if self.selected == state.last_selected {
            return;
        }

        let now = Instant::now();
        if self.selected {
            // Rising edge. Reset the stroke to zero so a quick reselect always
            // re-traces, then start both the draw-in and the alpha ramp.
            state.stroke = Hover::settled(false).with_fade(style::EMPHASIS);
            state.stroke.flip(true, now);
            state.alpha.flip(true, now);
        } else {
            // Falling edge. Leave the stroke fully drawn and fade the whole
            // glyph out.
            state.alpha.flip(false, now);
        }

        state.last_selected = self.selected;
        state.last_factor.set(None);
        state.last_alpha.set(None);
        state.last_primary.set(None);
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
        let Event::Window(window::Event::RedrawRequested(_)) = event else {
            return;
        };

        let state = tree.state.downcast_ref::<State>();
        let now = Instant::now();
        let animating = state.stroke.animating(now) || state.alpha.animating(now);

        // One extra redraw on the animating-to-settled edge lands the cache on
        // the exact target factors.
        if animating || state.was_animating.get() {
            shell.request_redraw();
        }

        state.was_animating.set(animating);
    }
}

impl<'a, Message: 'a> From<AnimatedTick<'a, Message>> for Element<'a, Message> {
    fn from(tick: AnimatedTick<'a, Message>) -> Self {
        Element::new(tick)
    }
}
