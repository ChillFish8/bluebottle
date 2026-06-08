use std::ops::RangeInclusive;

use iced::advanced::renderer::{self, Quad};
use iced::advanced::widget::tree::{self, Tree};
use iced::advanced::{Clipboard, Layout, Renderer, Shell, Widget, layout};
use iced::time::Instant;
use iced::widget::Row;
use iced::{
    Background,
    Border,
    Center,
    Element,
    Event,
    Length,
    Point,
    Rectangle,
    Size,
    mouse,
    window,
};

use crate::animate::hover::Hover;
use crate::border::Radius;
use crate::util::lerp;
use crate::widget::text;
use crate::{color, font, spacing, style};

const TRACK_HEIGHT: f32 = 6.0;
const THUMB_DIAMETER: f32 = 18.0;
const DRAG_SCALE: f32 = 1.06;
/// Reserved inset on each side of the track. Sized to the drag-grown thumb so
/// the thumb never spills past the widget bounds at the range endpoints.
const THUMB_INSET: f32 = THUMB_DIAMETER * DRAG_SCALE / 2.0;
const HIT_HEIGHT: f32 = 24.0;
const FILL_ALPHA: f32 = 0.62;
const TICK_HEIGHT: f32 = 6.0;
const TICK_WIDTH: f32 = 1.0;
const TICK_ALPHA: f32 = 0.30;
/// Hard upper bound on tick marks per render. Any stepped slider whose
/// span/step would exceed this cap stops drawing ticks rather than freezing
/// the draw loop. The cap is well above what a human can resolve.
const MAX_TICK_COUNT: i32 = 200;
const READOUT_BUDGET: f32 = 56.0;
const LEAD_ICON_SIZE: f32 = 18.0;

/// A continuous range control. Bordered glass track, accent glass fill, solid
/// white thumb.
pub struct Slider<'a, Message> {
    value: f32,
    range: RangeInclusive<f32>,
    step: f32,
    width: Length,
    disabled: bool,
    on_change: Box<dyn Fn(f32) -> Message + 'a>,
    on_release: Option<Message>,
    lead: Option<Lead<'a>>,
}

struct Lead<'a> {
    icon: &'a str,
    format: Box<dyn Fn(f32) -> String + 'a>,
}

/// Build a [`Slider`] over `0.0..=1.0`. `on_change` fires while dragging.
pub fn slider<'a, Message>(
    value: f32,
    on_change: impl Fn(f32) -> Message + 'a,
) -> Slider<'a, Message> {
    Slider {
        value,
        range: 0.0..=1.0,
        step: 0.0,
        width: Length::Fill,
        disabled: false,
        on_change: Box::new(on_change),
        on_release: None,
        lead: None,
    }
}

impl<'a, Message> Slider<'a, Message> {
    /// Sets the value range. Defaults to `0.0..=1.0`.
    pub fn range(mut self, range: RangeInclusive<f32>) -> Self {
        self.range = range;
        self
    }

    /// Snaps the value to multiples of `step` and draws faint ticks. A `step`
    /// of zero leaves the slider continuous.
    pub fn step(mut self, step: f32) -> Self {
        self.step = step.max(0.0);
        self
    }

    /// Sets the component width.
    pub fn width(mut self, width: impl Into<Length>) -> Self {
        self.width = width.into();
        self
    }

    /// Adds a leading icon and a trailing mono readout, turning the bare
    /// slider into a labelled row. The format closure renders the value.
    pub fn lead_icon(
        mut self,
        icon: &'a str,
        format: impl Fn(f32) -> String + 'a,
    ) -> Self {
        self.lead = Some(Lead {
            icon,
            format: Box::new(format),
        });
        self
    }

    /// Fires when the user releases the thumb.
    pub fn on_release(mut self, message: Message) -> Self {
        self.on_release = Some(message);
        self
    }

    /// Drops the whole control to forty percent and detaches the drag handler.
    pub fn disabled(mut self, on: bool) -> Self {
        self.disabled = on;
        self
    }
}

#[derive(Default)]
struct State {
    hover: Hover,
    drag_scale: Hover,
    drag: Option<DragOrigin>,
}

#[derive(Clone, Copy)]
struct DragOrigin {
    start_value: f32,
    start_x: f32,
}

struct Rail<'a, Message> {
    value: f32,
    range: RangeInclusive<f32>,
    step: f32,
    width: Length,
    disabled: bool,
    on_change: Box<dyn Fn(f32) -> Message + 'a>,
    on_release: Option<Message>,
}

impl<Message> Rail<'_, Message> {
    fn normalized(&self) -> f32 {
        normalize(self.value, &self.range)
    }

    fn snap(&self, value: f32) -> f32 {
        let start = *self.range.start();
        if !value.is_finite() {
            return start;
        }

        let clamped = value.clamp(start, *self.range.end());

        if self.step <= 0.0 {
            clamped
        } else {
            let snapped = start + ((clamped - start) / self.step).round() * self.step;
            snapped.clamp(start, *self.range.end())
        }
    }

    fn alpha_factor(&self) -> f32 {
        if self.disabled { 0.40 } else { 1.0 }
    }
}

fn normalize(value: f32, range: &RangeInclusive<f32>) -> f32 {
    let span = range.end() - range.start();
    if !span.is_finite() || span <= 0.0 || !value.is_finite() {
        return 0.0;
    }
    ((value - range.start()) / span).clamp(0.0, 1.0)
}

fn track_rect(bounds: Rectangle) -> Rectangle {
    Rectangle {
        x: bounds.x + THUMB_INSET,
        y: bounds.y + (bounds.height - TRACK_HEIGHT) / 2.0,
        width: (bounds.width - THUMB_INSET * 2.0).max(0.0),
        height: TRACK_HEIGHT,
    }
}

impl<Message> Widget<Message, iced::Theme, iced::Renderer> for Rail<'_, Message>
where
    Message: Clone,
{
    fn size(&self) -> Size<Length> {
        Size {
            width: self.width,
            height: Length::Fixed(HIT_HEIGHT),
        }
    }

    fn layout(
        &mut self,
        _tree: &mut Tree,
        _renderer: &iced::Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        layout::atomic(limits, self.width, Length::Fixed(HIT_HEIGHT))
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
        let track = track_rect(bounds);
        let radius = Radius::new(track.height / 2.0);
        let alpha = self.alpha_factor();

        renderer.fill_quad(
            Quad {
                bounds: track,
                border: style::hairline(color::fade(color::border_strong(), alpha))
                    .rounded(radius),
                ..Quad::default()
            },
            Background::Color(color::fade(color::hover_veil(), alpha)),
        );

        if self.step > 0.0 {
            draw_ticks(renderer, &track, &self.range, self.step, alpha);
        }

        let factor = self.normalized();
        let fill_width = track.width * factor;
        if fill_width > 0.0 {
            let fill = Rectangle {
                x: track.x,
                y: track.y,
                width: fill_width,
                height: track.height,
            };
            let fill_color =
                color::with_alpha(color::primary(), color::srgb_alpha(FILL_ALPHA));

            renderer.fill_quad(
                Quad {
                    bounds: fill,
                    border: style::hairline(color::fade(color::primary(), alpha))
                        .rounded(radius),
                    ..Quad::default()
                },
                Background::Color(color::fade(fill_color, alpha)),
            );
        }

        let now = Instant::now();
        let hover_factor = state.hover.current(now);
        let drag_factor = state.drag_scale.current(now);
        let scale = lerp(1.0, DRAG_SCALE, drag_factor);
        let thumb_diameter = THUMB_DIAMETER * scale;
        let thumb_centre =
            Point::new(track.x + fill_width, track.y + track.height / 2.0);
        let thumb_bounds = Rectangle {
            x: thumb_centre.x - thumb_diameter / 2.0,
            y: thumb_centre.y - thumb_diameter / 2.0,
            width: thumb_diameter,
            height: thumb_diameter,
        };
        let thumb_border = Border::default().rounded(thumb_diameter / 2.0);

        let lift = lerp(0.6, 1.0, hover_factor.max(drag_factor));
        renderer.fill_quad(
            Quad {
                bounds: thumb_bounds,
                border: thumb_border,
                shadow: style::scale_shadow(style::ELEVATION_RESTING, lift * alpha),
                ..Quad::default()
            },
            Background::Color(color::fade(color::WHITE, alpha)),
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
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        _renderer: &iced::Renderer,
        _clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
        _viewport: &Rectangle,
    ) {
        let bounds = layout.bounds();
        let track = track_rect(bounds);
        let state = tree.state.downcast_mut::<State>();
        let over = cursor.is_over(bounds);
        let now = Instant::now();

        if self.disabled {
            if state.hover.flip(false, now) {
                shell.request_redraw();
            }
            if state.drag_scale.flip(false, now) {
                shell.request_redraw();
            }
            return;
        }

        match event {
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left))
                if over && !shell.is_event_captured() =>
            {
                if let Some(point) = cursor.position() {
                    let next = self.snap(value_at(point.x, &track, &self.range));
                    state.drag = Some(DragOrigin {
                        start_value: next,
                        start_x: point.x,
                    });
                    state.drag_scale.flip(true, now);

                    if (next - self.value).abs() > f32::EPSILON {
                        shell.publish((self.on_change)(next));
                    }

                    shell.capture_event();
                    shell.request_redraw();
                }
            },

            Event::Mouse(mouse::Event::CursorMoved { .. }) => {
                if let Some(origin) = state.drag
                    && let Some(point) = cursor.position()
                {
                    let span = self.range.end() - self.range.start();
                    let delta = if track.width > 0.0 {
                        (point.x - origin.start_x) / track.width * span
                    } else {
                        0.0
                    };

                    let next = self.snap(origin.start_value + delta);
                    if (next - self.value).abs() > f32::EPSILON {
                        shell.publish((self.on_change)(next));
                    }

                    shell.request_redraw();
                }
            },

            Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)) => {
                if state.drag.take().is_some() {
                    state.drag_scale.flip(false, now);
                    if let Some(message) = self.on_release.clone() {
                        shell.publish(message);
                    }

                    shell.request_redraw();
                }
            },

            _ => {
                let active = over || state.drag.is_some();
                if state.hover.flip(active, now) {
                    shell.request_redraw();
                }

                if let Event::Window(window::Event::RedrawRequested(_)) = event
                    && (state.hover.animating(now) || state.drag_scale.animating(now))
                {
                    shell.request_redraw();
                }
            },
        }
    }

    fn mouse_interaction(
        &self,
        _tree: &Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        _viewport: &Rectangle,
        _renderer: &iced::Renderer,
    ) -> mouse::Interaction {
        if !self.disabled && cursor.is_over(layout.bounds()) {
            mouse::Interaction::Pointer
        } else {
            mouse::Interaction::None
        }
    }
}

fn value_at(x: f32, track: &Rectangle, range: &RangeInclusive<f32>) -> f32 {
    let span = range.end() - range.start();
    if track.width <= 0.0 || span <= 0.0 {
        return *range.start();
    }
    let local = ((x - track.x) / track.width).clamp(0.0, 1.0);
    range.start() + local * span
}

fn draw_ticks(
    renderer: &mut iced::Renderer,
    track: &Rectangle,
    range: &RangeInclusive<f32>,
    step: f32,
    alpha: f32,
) {
    let span = range.end() - range.start();
    if !span.is_finite() || span <= 0.0 || !step.is_finite() || step <= 0.0 {
        return;
    }

    // The cast saturates above i32::MAX, so a tiny step against a finite span
    // would otherwise issue billions of fill_quad calls per frame. The hard
    // cap stops drawing past a human-resolvable density.
    let raw = (span / step).round();
    if !raw.is_finite() || raw <= 0.0 {
        return;
    }

    let count = (raw as i32).min(MAX_TICK_COUNT);
    if count <= 0 {
        return;
    }

    let y = track.y + (track.height - TICK_HEIGHT) / 2.0;
    let tint = color::fade(
        color::with_alpha(color::WHITE, color::srgb_alpha(TICK_ALPHA)),
        alpha,
    );

    for i in 0..=count {
        let t = i as f32 / count as f32;
        let x = track.x + track.width * t - TICK_WIDTH / 2.0;
        renderer.fill_quad(
            Quad {
                bounds: Rectangle {
                    x,
                    y,
                    width: TICK_WIDTH,
                    height: TICK_HEIGHT,
                },
                ..Quad::default()
            },
            Background::Color(tint),
        );
    }
}

impl<'a, Message> From<Slider<'a, Message>> for Element<'a, Message>
where
    Message: Clone + 'a,
{
    fn from(s: Slider<'a, Message>) -> Self {
        let rail: Element<'a, Message> = Element::new(Rail {
            value: s.value,
            range: s.range.clone(),
            step: s.step,
            width: if s.lead.is_some() {
                Length::Fill
            } else {
                s.width
            },
            disabled: s.disabled,
            on_change: s.on_change,
            on_release: s.on_release,
        });

        let Some(lead) = s.lead else {
            return rail;
        };

        let icon =
            crate::icon::filled(lead.icon)
                .size(LEAD_ICON_SIZE)
                .color(color::fade(
                    color::TEXT_PRIMARY,
                    if s.disabled { 0.40 } else { 1.0 },
                ));
        let readout_text = (lead.format)(s.value);
        let readout = text::caption(readout_text)
            .font(font::mono_medium())
            .size(12)
            .color(color::fade(
                color::TEXT_PRIMARY,
                if s.disabled { 0.40 } else { 1.0 },
            ))
            .width(Length::Fixed(READOUT_BUDGET))
            .align_x(iced::alignment::Horizontal::Right);

        Row::with_children([icon.into(), rail, readout.into()])
            .width(s.width)
            .align_y(Center)
            .spacing(spacing::GAP_12)
            .into()
    }
}
