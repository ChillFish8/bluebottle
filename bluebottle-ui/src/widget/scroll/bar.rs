//! The Firefox-style overlay scrollbar shared by every vertical scroll widget.
//!
//! The bar is transparent at rest, reveals while scrolling, and fades out
//! after a pause. Hovering its edge widens it into a grabbable thumb. The
//! struct only owns the bar's visual state. Offset and content layout stay
//! with the parent widget.

use std::time::{Duration, Instant};

use iced::advanced::Shell;
use iced::advanced::renderer::Quad;
use iced::{Event, Rectangle, border, mouse, window};

use crate::{color, easing};

/// Pixels moved per line of wheel scroll, matching iced.
pub(crate) const LINE: f32 = 60.0;

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

/// What the user did with the scrollbar this frame.
#[derive(Clone, Copy)]
pub(crate) enum BarEvent {
    /// Wheel input, signed pixels. Apply by subtracting from the offset.
    Wheel(f32),
    /// Thumb drag, the resulting absolute scroll offset in pixels.
    DragTo(f32),
    /// The bar consumed the event for hover, press, release, or a drag that
    /// did not move the offset. The parent must skip forwarding this event
    /// to child widgets to avoid double-clicks on items under the thumb.
    Captured,
    /// Nothing to apply to the offset this frame.
    None,
}

impl BarEvent {
    /// Whether the bar consumed the event, so the parent should stop
    /// propagating it to children.
    pub fn captured(self) -> bool {
        !matches!(self, BarEvent::None)
    }
}

#[derive(Clone, Copy)]
pub(crate) struct ScrollBar {
    /// Time of the last scroll, driving the reveal and fade.
    scrolled: Instant,
    /// Whether the pointer is over the bar's edge strip.
    hovering: bool,
    /// When `hovering` last changed, driving the widen ease.
    hover_changed: Instant,
    /// Cursor offset from the thumb's top while dragging it.
    drag: Option<f32>,
}

impl Default for ScrollBar {
    fn default() -> Self {
        let now = Instant::now();

        Self {
            scrolled: now - (HOLD + FADE),
            hovering: false,
            hover_changed: now - EXPAND,
            drag: None,
        }
    }
}

impl ScrollBar {
    /// Reveals the bar as if the content was just scrolled.
    pub fn note_scrolled(&mut self, now: Instant) {
        self.scrolled = now;
    }

    /// Whether any of the bar's own animations are still in flight.
    pub fn animating(&self, now: Instant) -> bool {
        let widening = now.duration_since(self.hover_changed) < EXPAND;
        let resting = !self.hovering && self.drag.is_none();
        let fading = resting && now.duration_since(self.scrolled) < HOLD + FADE;

        widening || fading || self.drag.is_some()
    }

    /// Translates an iced event into a [`BarEvent`] and updates the visual
    /// state. Captures the event when it belongs to the bar.
    pub fn update<Message>(
        &mut self,
        event: &Event,
        bounds: Rectangle,
        content_height: f32,
        offset: f32,
        cursor: mouse::Cursor,
        shell: &mut Shell<'_, Message>,
    ) -> BarEvent {
        let now = Instant::now();
        let max_offset = (content_height - bounds.height).max(0.0);

        // Drop an in-flight drag if the content shrank below the viewport so
        // the grab offset cannot resurface later with a stale position.
        if max_offset == 0.0 && self.drag.is_some() {
            self.drag = None;
        }

        match event {
            Event::Mouse(mouse::Event::WheelScrolled { delta })
                if max_offset > 0.0 && cursor.is_over(bounds) =>
            {
                let dy = match delta {
                    mouse::ScrollDelta::Lines { y, .. } => y * LINE,
                    mouse::ScrollDelta::Pixels { y, .. } => *y,
                };

                let moved = (offset - dy).clamp(0.0, max_offset);

                if moved == offset {
                    return BarEvent::None;
                }

                self.scrolled = now;
                shell.request_redraw();
                // Do not call shell.capture_event() for wheel events.
                // iced's UserInterface drops its cached overlay whenever
                // the root sees a captured event, which would make any
                // overlay (e.g. a DropDown) flicker once per wheel tick.
                BarEvent::Wheel(dy)
            },

            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left))
                if max_offset > 0.0 =>
            {
                let width = self.width(now);
                let bar = thumb(bounds, content_height, offset, max_offset, width);

                if let Some(position) = cursor.position()
                    && cursor.is_over(bar)
                {
                    self.drag = Some(position.y - bar.y);
                    self.scrolled = now;
                    shell.request_redraw();
                    shell.capture_event();
                    return BarEvent::Captured;
                }

                BarEvent::None
            },

            Event::Mouse(mouse::Event::CursorMoved { .. }) if max_offset > 0.0 => {
                if cursor.is_over(bounds) {
                    self.scrolled = now;
                    shell.request_redraw();
                }

                let edge = THIN.max(WIDE) + PAD;
                let strip = Rectangle {
                    x: bounds.x + bounds.width - edge,
                    width: edge,
                    ..bounds
                };

                let over = cursor.is_over(strip);
                if over != self.hovering {
                    self.hovering = over;
                    self.hover_changed = now;
                    shell.request_redraw();
                }

                if let Some(grab) = self.drag
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

                    let new_offset = fraction * max_offset;
                    self.scrolled = now;
                    shell.request_redraw();
                    shell.capture_event();
                    return BarEvent::DragTo(new_offset);
                }

                BarEvent::None
            },

            Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left))
                if self.drag.is_some() =>
            {
                self.drag = None;
                shell.request_redraw();
                shell.capture_event();
                BarEvent::Captured
            },

            Event::Window(window::Event::RedrawRequested(_)) if self.animating(now) => {
                shell.request_redraw();
                BarEvent::None
            },

            _ => BarEvent::None,
        }
    }

    /// Paints the thumb if it is currently visible.
    pub fn draw(
        &self,
        renderer: &mut iced::Renderer,
        bounds: Rectangle,
        content_height: f32,
        offset: f32,
    ) {
        use iced::advanced::Renderer as _;

        let max_offset = (content_height - bounds.height).max(0.0);
        if max_offset <= 0.0 {
            return;
        }

        let now = Instant::now();
        let reveal = self.reveal(now);
        if reveal <= EPSILON {
            return;
        }

        let factor = self.hover_factor(now);
        let width = THIN + (WIDE - THIN) * factor;
        let bar = thumb(bounds, content_height, offset, max_offset, width);

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

    /// Cursor interaction when the pointer is over the bar.
    pub fn mouse_interaction(
        &self,
        bounds: Rectangle,
        content_height: f32,
        offset: f32,
        cursor: mouse::Cursor,
    ) -> Option<mouse::Interaction> {
        let max_offset = (content_height - bounds.height).max(0.0);
        if max_offset <= 0.0 {
            return None;
        }

        if self.drag.is_some() {
            return Some(mouse::Interaction::Grabbing);
        }

        let now = Instant::now();
        let width = self.width(now);
        let bar = thumb(bounds, content_height, offset, max_offset, width);

        cursor.is_over(bar).then_some(mouse::Interaction::Grab)
    }

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
