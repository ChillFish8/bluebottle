//! A vertical scroll area with a Firefox-style overlay scrollbar.
//!
//! The bar is transparent at rest. It reveals while scrolling and fades out
//! after a pause. Hovering its edge widens it into a grabbable thumb. The
//! widget drives its own fade and widen animations, so the application does
//! not tick it.
//!
//! Beyond the bar the widget also supports animated programmatic scroll via
//! [`Scrollable::scroll_to`] and a user-scroll callback via
//! [`Scrollable::on_scroll`]. Content that grows or shrinks underneath the
//! viewport keeps the user's perceived position thanks to a one-anchor
//! reflow check on every frame.

use std::f32::consts::PI;
use std::time::Instant;

use iced::advanced::renderer::Quad;
use iced::advanced::widget::{Operation, Tree, tree};
use iced::advanced::{Clipboard, Layout, Renderer, Shell, Widget, layout, renderer};
use iced::{
    Background,
    Color,
    Element,
    Event,
    Length,
    Radians,
    Rectangle,
    Size,
    Transformation,
    Vector,
    gradient,
    mouse,
    window,
};

use crate::widget::scroll::ScrollEngine;

/// Vertical band painted at each edge while scrolling, in logical pixels.
const FADE_HEIGHT: f32 = 18.0;

/// Scroll distance over which the fade alpha ramps in from zero.
const FADE_RAMP: f32 = 14.0;

/// Below this the fade counts as fully hidden.
const FADE_EPSILON: f32 = 0.001;

/// Makes `content` scroll vertically on overflow.
pub fn scrollable<'a, Message>(
    content: impl Into<Element<'a, Message>>,
) -> Scrollable<'a, Message> {
    Scrollable {
        content: content.into(),
        width: Length::Fill,
        height: Length::Fill,
        max_height: None,
        scroll_to: None,
        on_scroll: None,
        fade_color: None,
    }
}

/// A vertical scroll area, built by [`scrollable`].
pub struct Scrollable<'a, Message> {
    content: Element<'a, Message>,
    width: Length,
    height: Length,
    max_height: Option<f32>,
    scroll_to: Option<f32>,
    on_scroll: Option<Box<dyn Fn(f32) -> Message + 'a>>,
    fade_color: Option<Color>,
}

impl<'a, Message> Scrollable<'a, Message> {
    /// Sets the width of the scroll area.
    pub fn width(mut self, width: impl Into<Length>) -> Self {
        self.width = width.into();
        self
    }

    /// Sets the height of the scroll area.
    pub fn height(mut self, height: impl Into<Length>) -> Self {
        self.height = height.into();
        self
    }

    /// Caps the scroll area's height. The widget shrinks to its content
    /// when it fits and tops out at `max_height` when it does not. Useful
    /// for menus that want a hard row-count budget before scrolling.
    pub fn max_height(mut self, max_height: f32) -> Self {
        self.max_height = Some(max_height);
        self
    }

    /// Animates the offset toward a pixel position. Passing `None` cancels
    /// any active animation and leaves the offset where it is.
    pub fn scroll_to(mut self, offset: Option<f32>) -> Self {
        self.scroll_to = offset;
        self
    }

    /// Called with the new offset after the user moves the scroll position.
    /// Programmatic scrolls from [`Scrollable::scroll_to`] do not fire it.
    pub fn on_scroll(mut self, on_scroll: impl Fn(f32) -> Message + 'a) -> Self {
        self.on_scroll = Some(Box::new(on_scroll));
        self
    }

    /// Paints a vertical fade at each edge while scrolling. The fade reads
    /// as the rows dissolving into `color`. The top edge hides at the top
    /// of the content, the bottom edge hides at the bottom.
    pub fn fade_edges(mut self, color: Color) -> Self {
        self.fade_color = Some(color);
        self
    }
}

impl<'a, Message> From<Scrollable<'a, Message>> for Element<'a, Message>
where
    Message: 'a,
{
    fn from(scrollable: Scrollable<'a, Message>) -> Self {
        Element::new(scrollable)
    }
}

#[derive(Clone, Copy, Default)]
struct State {
    engine: ScrollEngine,
    /// Previous frame's `scroll_to`, used to detect a fresh request.
    last_request: Option<f32>,
}

impl<'a, Message> Widget<Message, iced::Theme, iced::Renderer>
    for Scrollable<'a, Message>
{
    fn size(&self) -> Size<Length> {
        // Only override Fill-style heights to Shrink when max_height is set,
        // so a caller-supplied Length::Fixed or Length::Shrink is honoured.
        let height = match self.height {
            Length::Fill | Length::FillPortion(_) if self.max_height.is_some() => {
                Length::Shrink
            },
            other => other,
        };
        Size {
            width: self.width,
            height,
        }
    }

    fn layout(
        &mut self,
        tree: &mut Tree,
        renderer: &iced::Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        let mut limits = limits.width(self.width).height(self.height);
        if let Some(max) = self.max_height {
            limits = limits.max_height(max);
        }
        let viewport = limits.max();

        let child = self.content.as_widget_mut().layout(
            &mut tree.children[0],
            renderer,
            &layout::Limits::new(Size::ZERO, Size::new(viewport.width, f32::INFINITY)),
        );

        let height = match self.max_height {
            Some(_) => child.bounds().height.min(viewport.height),
            None => viewport.height,
        };

        layout::Node::with_children(Size::new(viewport.width, height), vec![child])
    }

    fn draw(
        &self,
        tree: &Tree,
        renderer: &mut iced::Renderer,
        theme: &iced::Theme,
        style: &renderer::Style,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        _viewport: &Rectangle,
    ) {
        let bounds = layout.bounds();
        let child = layout.children().next().unwrap();
        let content_height = child.bounds().height;

        let state = tree.state.downcast_ref::<State>();
        let offset = state.engine.offset;
        let shift = Transformation::translate(0.0, offset);

        renderer.with_layer(bounds, |renderer| {
            renderer.with_translation(Vector::new(0.0, -offset), |renderer| {
                self.content.as_widget().draw(
                    &tree.children[0],
                    renderer,
                    theme,
                    style,
                    child,
                    cursor * shift,
                    &(bounds * shift),
                );
            });
        });

        // Fade and bar each ride on their own sub-layer so they composite
        // strictly on top of the content. iced's per-layer pipeline order
        // would otherwise draw the fade quads before the row text within
        // the same layer, and the text would punch through.
        if let Some(fade_color) = self.fade_color {
            renderer.with_layer(bounds, |renderer| {
                draw_edge_fades(renderer, bounds, content_height, offset, fade_color);
            });
        }

        renderer.with_layer(bounds, |renderer| {
            state
                .engine
                .bar
                .draw(renderer, bounds, content_height, offset);
        });
    }

    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<State>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(State::default())
    }

    fn children(&self) -> Vec<Tree> {
        vec![Tree::new(&self.content)]
    }

    fn diff(&self, tree: &mut Tree) {
        tree.diff_children(std::slice::from_ref(&self.content));

        let state = tree.state.downcast_mut::<State>();
        if state.last_request != self.scroll_to {
            state.last_request = self.scroll_to;
            match self.scroll_to {
                Some(_) => state.engine.start_target(Instant::now()),
                None => state.engine.clear_target(),
            }
        }
    }

    fn operate(
        &mut self,
        tree: &mut Tree,
        layout: Layout<'_>,
        renderer: &iced::Renderer,
        operation: &mut dyn Operation,
    ) {
        let child = layout.children().next().unwrap();

        self.content.as_widget_mut().operate(
            &mut tree.children[0],
            child,
            renderer,
            operation,
        );
    }

    fn update(
        &mut self,
        tree: &mut Tree,
        event: &Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &iced::Renderer,
        clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
        viewport: &Rectangle,
    ) {
        let bounds = layout.bounds();
        let child = layout.children().next().unwrap();
        let content_height = child.bounds().height;
        let max_offset = (content_height - bounds.height).max(0.0);

        let now = Instant::now();
        let state = tree.state.downcast_mut::<State>();

        state.engine.clamp(max_offset);

        let bar_event = state.engine.bar.update(
            event,
            bounds,
            content_height,
            state.engine.offset,
            cursor,
            shell,
        );
        let captured = bar_event.captured();

        let moved = state.engine.apply_bar_event(bar_event, max_offset, now);

        if moved && let Some(on_scroll) = &self.on_scroll {
            shell.publish(on_scroll(state.engine.offset));
        }

        if let Event::Window(window::Event::RedrawRequested(_)) = event {
            if let Some(target) = self.scroll_to {
                state.engine.step_target(now, target, max_offset);
            }
            if state.engine.animating(now) {
                shell.request_redraw();
            }
        }

        if captured {
            return;
        }

        let shift = Transformation::translate(0.0, state.engine.offset);

        // Mask the cursor to Unavailable when it sits outside the visible
        // viewport. Without this, children whose layout positions lie above
        // or below the clipped band still hit-test true wherever the
        // shifted cursor lands on them, so off-viewport rows would react
        // to clicks aimed at empty space.
        let content_cursor = if cursor.is_over(bounds) {
            cursor * shift
        } else {
            mouse::Cursor::Unavailable
        };

        self.content.as_widget_mut().update(
            &mut tree.children[0],
            event,
            child,
            content_cursor,
            renderer,
            clipboard,
            shell,
            &(*viewport * shift),
        );
    }

    fn mouse_interaction(
        &self,
        tree: &Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
        renderer: &iced::Renderer,
    ) -> mouse::Interaction {
        let bounds = layout.bounds();
        let child = layout.children().next().unwrap();
        let content_height = child.bounds().height;

        let state = tree.state.downcast_ref::<State>();
        let offset = state.engine.offset;

        if let Some(interaction) =
            state
                .engine
                .bar
                .mouse_interaction(bounds, content_height, offset, cursor)
        {
            return interaction;
        }

        let shift = Transformation::translate(0.0, offset);

        // Match update()'s viewport-cursor mask so the pointer style only
        // tracks rows that are actually visible.
        let content_cursor = if cursor.is_over(bounds) {
            cursor * shift
        } else {
            mouse::Cursor::Unavailable
        };

        self.content.as_widget().mouse_interaction(
            &tree.children[0],
            child,
            content_cursor,
            &(*viewport * shift),
            renderer,
        )
    }

    fn overlay<'b>(
        &'b mut self,
        tree: &'b mut Tree,
        layout: Layout<'b>,
        renderer: &iced::Renderer,
        viewport: &Rectangle,
        translation: Vector,
    ) -> Option<iced::advanced::overlay::Element<'b, Message, iced::Theme, iced::Renderer>>
    {
        let child = layout.children().next().unwrap();
        let state = tree.state.downcast_ref::<State>();
        let offset = state.engine.offset;

        self.content.as_widget_mut().overlay(
            &mut tree.children[0],
            child,
            renderer,
            viewport,
            translation - Vector::new(0.0, offset),
        )
    }
}

/// Paints the top and bottom fades. Each band ramps in over the first
/// `FADE_RAMP` pixels of scroll travel at its end of the content, so the
/// rows read as dissolving into the menu surface instead of clipping. The
/// band height is also capped at half the viewport so the two bands cannot
/// overlap and stack into a solid wall when the scrollable is short.
fn draw_edge_fades(
    renderer: &mut iced::Renderer,
    bounds: Rectangle,
    content_height: f32,
    offset: f32,
    color: Color,
) {
    let max_offset = (content_height - bounds.height).max(0.0);
    if max_offset <= 0.0 {
        return;
    }

    let band_height = FADE_HEIGHT.min(bounds.height / 2.0);
    if band_height <= 0.0 {
        return;
    }

    let top_factor = (offset / FADE_RAMP).clamp(0.0, 1.0);
    let bottom_factor = ((max_offset - offset) / FADE_RAMP).clamp(0.0, 1.0);

    if top_factor > FADE_EPSILON {
        let band = Rectangle {
            x: bounds.x,
            y: bounds.y,
            width: bounds.width,
            height: band_height,
        };
        draw_fade_band(renderer, band, color, top_factor, true);
    }

    if bottom_factor > FADE_EPSILON {
        let band = Rectangle {
            x: bounds.x,
            y: bounds.y + bounds.height - band_height,
            width: bounds.width,
            height: band_height,
        };
        draw_fade_band(renderer, band, color, bottom_factor, false);
    }
}

/// Paints one fade band as a single linear-gradient quad. `opaque_at_top`
/// reads the clip edge from the top of `band` when true, from the bottom
/// when false. The whole band's intensity is multiplied by `factor`. A real
/// gradient avoids the visible stair-step banding a stripe stack produces.
fn draw_fade_band(
    renderer: &mut iced::Renderer,
    band: Rectangle,
    color: Color,
    factor: f32,
    opaque_at_top: bool,
) {
    let near = Color {
        a: color.a * factor,
        ..color
    };
    let far = Color { a: 0.0, ..color };

    let (top, bottom) = if opaque_at_top {
        (near, far)
    } else {
        (far, near)
    };

    let gradient = gradient::Linear::new(Radians(PI))
        .add_stop(0.0, top)
        .add_stop(1.0, bottom);

    renderer.fill_quad(
        Quad {
            bounds: band,
            ..Quad::default()
        },
        Background::Gradient(gradient.into()),
    );
}
