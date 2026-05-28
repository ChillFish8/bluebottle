//! A vertical scroll area with a Firefox-style overlay scrollbar.
//!
//! The bar is transparent at rest. It reveals while scrolling and fades out
//! after a pause. Hovering its edge widens it into a grabbable thumb. The widget
//! drives its own fade and widen animations, so the application does not tick it.

use std::time::{Duration, Instant};

use iced::advanced::renderer::{Quad, Style};
use iced::advanced::widget::{Operation, Tree, tree};
use iced::advanced::{Clipboard, Layout, Renderer, Shell, Widget, layout};
use iced::{
    Element,
    Event,
    Length,
    Rectangle,
    Size,
    Transformation,
    Vector,
    border,
    mouse,
    window,
};

use crate::{color, easing};

/// Pixels moved per line of wheel scroll, matching iced.
const LINE: f32 = 60.0;

/// The bar's width at rest, in logical pixels.
const THIN: f32 = 2.0;

/// The bar's width while hovered.
const WIDE: f32 = 6.0;

/// Shortest the thumb is allowed to get on tall content.
const MIN_THUMB: f32 = 28.0;

/// Gap between the bar and the right edge, so it sits just off the edge.
const PAD: f32 = 2.0;

/// How long the bar stays fully visible after a scroll before fading.
const HOLD: Duration = Duration::from_millis(700);

/// How long the bar takes to fade out once the hold passes.
const FADE: Duration = Duration::from_millis(400);

/// How long the widen on hover takes.
const EXPAND: Duration = Duration::from_millis(120);

/// Below this the bar counts as fully hidden.
const EPSILON: f32 = 0.001;

/// Makes `content` scroll vertically on overflow.
pub fn scrollable<'a, Message>(
    content: impl Into<Element<'a, Message>>,
) -> Scrollable<'a, Message> {
    Scrollable {
        content: content.into(),
        width: Length::Fill,
        height: Length::Fill,
    }
}

/// A vertical scroll area, built by [`scrollable`].
pub struct Scrollable<'a, Message> {
    content: Element<'a, Message>,
    width: Length,
    height: Length,
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
}

impl<'a, Message> From<Scrollable<'a, Message>> for Element<'a, Message>
where
    Message: 'a,
{
    fn from(scrollable: Scrollable<'a, Message>) -> Self {
        Element::new(scrollable)
    }
}

#[derive(Clone, Copy)]
struct State {
    /// Pixels the content is scrolled down by.
    offset: f32,
    /// Time of the last scroll, driving the reveal and fade.
    scrolled: Instant,
    /// Whether the pointer is over the bar's edge strip.
    hovering: bool,
    /// When `hovering` last changed, driving the widen ease.
    hover_changed: Instant,
    /// Cursor offset from the thumb's top while dragging it.
    drag: Option<f32>,
}

impl Default for State {
    fn default() -> Self {
        let now = Instant::now();

        Self {
            offset: 0.0,
            scrolled: now - (HOLD + FADE),
            hovering: false,
            hover_changed: now - EXPAND,
            drag: None,
        }
    }
}

impl State {
    /// How visible the bar is right now, 0.0 hidden to 1.0 solid.
    fn reveal(&self, now: Instant) -> f32 {
        if self.hovering || self.drag.is_some() {
            return 1.0;
        }

        let since = now.duration_since(self.scrolled);

        if since < HOLD {
            1.0
        } else if since < HOLD + FADE {
            1.0 - (since - HOLD).as_secs_f32() / FADE.as_secs_f32()
        } else {
            0.0
        }
    }

    /// How expanded the bar is right now, 0.0 thin to 1.0 wide.
    fn hover_factor(&self, now: Instant) -> f32 {
        let raw = (now.duration_since(self.hover_changed).as_secs_f32()
            / EXPAND.as_secs_f32())
        .clamp(0.0, 1.0);

        let eased = easing::STANDARD.y_at_x(raw);

        if self.hovering || self.drag.is_some() {
            eased
        } else {
            1.0 - eased
        }
    }

    /// The bar's width right now, eased between thin and wide.
    fn width(&self, now: Instant) -> f32 {
        THIN + (WIDE - THIN) * self.hover_factor(now)
    }

    /// Whether an animation is still in flight, so frames should keep coming.
    fn animating(&self, now: Instant) -> bool {
        let widening = now.duration_since(self.hover_changed) < EXPAND;
        let resting = !self.hovering && self.drag.is_none();
        let fading = resting && now.duration_since(self.scrolled) < HOLD + FADE;

        widening || fading || self.drag.is_some()
    }
}

/// The thumb's bounds for the given metrics.
fn thumb(
    bounds: Rectangle,
    content_height: f32,
    offset: f32,
    max_offset: f32,
    width: f32,
) -> Rectangle {
    let ratio = (bounds.height / content_height).min(1.0);
    let height = (bounds.height * ratio).clamp(MIN_THUMB, bounds.height);
    let span = bounds.height - height;
    let fraction = if max_offset > 0.0 {
        offset / max_offset
    } else {
        0.0
    };

    Rectangle {
        x: bounds.x + bounds.width - width - PAD,
        y: bounds.y + fraction * span,
        width,
        height,
    }
}

impl<'a, Message> Widget<Message, iced::Theme, iced::Renderer>
    for Scrollable<'a, Message>
{
    fn size(&self) -> Size<Length> {
        Size {
            width: self.width,
            height: self.height,
        }
    }

    fn layout(
        &mut self,
        tree: &mut Tree,
        renderer: &iced::Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        let limits = limits.width(self.width).height(self.height);
        let viewport = limits.max();

        // Lay the content out unbounded in height so it can overflow.
        let child = self.content.as_widget_mut().layout(
            &mut tree.children[0],
            renderer,
            &layout::Limits::new(Size::ZERO, Size::new(viewport.width, f32::INFINITY)),
        );

        layout::Node::with_children(viewport, vec![child])
    }

    fn draw(
        &self,
        tree: &Tree,
        renderer: &mut iced::Renderer,
        theme: &iced::Theme,
        style: &Style,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        _viewport: &Rectangle,
    ) {
        let bounds = layout.bounds();
        let child = layout.children().next().unwrap();
        let content_height = child.bounds().height;
        let max_offset = (content_height - bounds.height).max(0.0);

        let now = Instant::now();
        let state = tree.state.downcast_ref::<State>();
        let offset = state.offset.clamp(0.0, max_offset);
        let shift = Transformation::translate(0.0, offset);

        // Clip the content to the viewport and slide it up by the offset.
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

        if max_offset <= 0.0 {
            return;
        }

        let reveal = state.reveal(now);

        if reveal <= EPSILON {
            return;
        }

        let factor = state.hover_factor(now);
        let width = THIN + (WIDE - THIN) * factor;
        let bar = thumb(bounds, content_height, offset, max_offset, width);

        // Mid-gray when shown, lighter while hovered or grabbed.
        let tint = color::mix(color::SCROLLBAR, color::SCROLLBAR_HOVER, factor);

        renderer.fill_quad(
            Quad {
                bounds: bar,
                border: border::rounded(width / 2.0),
                ..Quad::default()
            },
            color::with_alpha(tint, tint.a * reveal),
        );
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
        state.offset = state.offset.clamp(0.0, max_offset);

        let mut handled = false;

        match event {
            Event::Mouse(mouse::Event::WheelScrolled { delta })
                if max_offset > 0.0 && cursor.is_over(bounds) =>
            {
                let dy = match delta {
                    mouse::ScrollDelta::Lines { y, .. } => y * LINE,
                    mouse::ScrollDelta::Pixels { y, .. } => *y,
                };

                let moved = (state.offset - dy).clamp(0.0, max_offset);

                if moved != state.offset {
                    state.offset = moved;
                    state.scrolled = now;
                    shell.request_redraw();
                    shell.capture_event();
                    handled = true;
                }
            },

            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left))
                if max_offset > 0.0 =>
            {
                let width = state.width(now);
                let bar = thumb(bounds, content_height, state.offset, max_offset, width);

                if let Some(position) = cursor.position()
                    && cursor.is_over(bar)
                {
                    state.drag = Some(position.y - bar.y);
                    state.scrolled = now;
                    shell.request_redraw();
                    shell.capture_event();
                    handled = true;
                }
            },

            Event::Mouse(mouse::Event::CursorMoved { .. }) if max_offset > 0.0 => {
                // Reveal the bar on pointer activity over the area, then let it fade.
                if cursor.is_over(bounds) {
                    state.scrolled = now;
                    shell.request_redraw();
                }

                // Cover the visible bar and its edge gap so hovering registers.
                let edge = THIN.max(WIDE) + PAD;
                let strip = Rectangle {
                    x: bounds.x + bounds.width - edge,
                    width: edge,
                    ..bounds
                };

                let over = cursor.is_over(strip);
                if over != state.hovering {
                    state.hovering = over;
                    state.hover_changed = now;
                    shell.request_redraw();
                }

                if let Some(grab) = state.drag
                    && let Some(position) = cursor.position()
                {
                    let ratio = (bounds.height / content_height).min(1.0);
                    let height = (bounds.height * ratio).clamp(MIN_THUMB, bounds.height);
                    let span = bounds.height - height;
                    let top = (position.y - grab).clamp(bounds.y, bounds.y + span);
                    let fraction = if span > 0.0 {
                        (top - bounds.y) / span
                    } else {
                        0.0
                    };

                    state.offset = fraction * max_offset;
                    state.scrolled = now;
                    shell.request_redraw();
                    shell.capture_event();
                    handled = true;
                }
            },

            Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left))
                if state.drag.is_some() =>
            {
                state.drag = None;
                shell.request_redraw();
                shell.capture_event();
                handled = true;
            },

            Event::Window(window::Event::RedrawRequested(_)) if state.animating(now) => {
                shell.request_redraw();
            },

            _ => {},
        }

        let offset = state.offset;

        if handled {
            return;
        }

        // Offset events into content space so inner widgets line up with the scrolled view.
        let shift = Transformation::translate(0.0, offset);

        self.content.as_widget_mut().update(
            &mut tree.children[0],
            event,
            child,
            cursor * shift,
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
        let max_offset = (content_height - bounds.height).max(0.0);

        let now = Instant::now();
        let state = tree.state.downcast_ref::<State>();

        if max_offset > 0.0 {
            if state.drag.is_some() {
                return mouse::Interaction::Grabbing;
            }

            let width = state.width(now);
            let bar = thumb(bounds, content_height, state.offset, max_offset, width);

            if cursor.is_over(bar) {
                return mouse::Interaction::Grab;
            }
        }

        let shift = Transformation::translate(0.0, state.offset);

        self.content.as_widget().mouse_interaction(
            &tree.children[0],
            child,
            cursor * shift,
            &(*viewport * shift),
            renderer,
        )
    }
}
