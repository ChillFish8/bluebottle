//! Picks Switcher. A muted eyebrow label paired with one dot per pick.
//! The active dot stretches into a 22x8 accent pill; the others sit as
//! 8x8 hairline-glass circles. Tapping a dot animates its width to the
//! pill across 240ms and emits the chosen index, twinning the fan deck's
//! promote semantics in compact chrome.
//!
//! `active` is a controlled prop. The consumer sets which pick is current
//! and the widget animates the width and colour of each dot toward that
//! state whenever the prop changes.

use std::borrow::Cow;
use std::time::{Duration, Instant};

use iced::advanced::renderer::Quad;
use iced::advanced::widget::{Tree, tree};
use iced::advanced::{Clipboard, Layout, Renderer, Shell, Widget, layout, renderer};
use iced::widget::Row;
use iced::{
    Border,
    Element,
    Event,
    Length,
    Point,
    Rectangle,
    Renderer as IcedRenderer,
    Size,
    Theme,
    alignment,
    mouse,
    window,
};

use crate::animate::hover::Hover;
use crate::widget::text;
use crate::{color, spacing};

const DOT_DIAMETER: f32 = 8.0;
const ACTIVE_WIDTH: f32 = 22.0;
const DOT_GAP: f32 = 6.0;
const PILL_RADIUS: f32 = DOT_DIAMETER * 0.5;
const PILL_FADE: Duration = Duration::from_millis(240);

const INACTIVE_ALPHA: f32 = 0.28;

const LABEL_LETTER_SPACING: f32 = 1.4;

/// Builds a Picks Switcher.
pub fn picks_switcher<'a, Message>(
    label: impl Into<Cow<'static, str>>,
    count: usize,
    active: usize,
) -> PicksSwitcher<'a, Message> {
    PicksSwitcher {
        label: label.into(),
        count,
        active,
        on_click: None,
    }
}

/// Builder for [`picks_switcher`].
pub struct PicksSwitcher<'a, Message> {
    label: Cow<'static, str>,
    count: usize,
    active: usize,
    on_click: Option<Box<dyn Fn(usize) -> Message + 'a>>,
}

impl<'a, Message> PicksSwitcher<'a, Message> {
    /// Sets the click handler. Fires with the index of the clicked dot.
    pub fn on_click(mut self, on_click: impl Fn(usize) -> Message + 'a) -> Self {
        self.on_click = Some(Box::new(on_click));
        self
    }
}

impl<'a, Message> From<PicksSwitcher<'a, Message>> for Element<'a, Message>
where
    Message: 'a,
{
    fn from(switcher: PicksSwitcher<'a, Message>) -> Self {
        let label = text::eyebrow(switcher.label.to_uppercase(), text::Variant::Main)
            .color(color::TEXT_SECONDARY)
            .letter_spacing(LABEL_LETTER_SPACING);

        // Clamp the active prop so an out-of-range value never leaves the
        // deck rendered with no pill while the layout still reserves the
        // wide-pill footprint.
        let count = switcher.count;
        let active = if count == 0 {
            0
        } else {
            switcher.active.min(count - 1)
        };

        let dots: Element<'a, Message> = Element::new(DotsWidget {
            count,
            active,
            on_click: switcher.on_click,
        });

        Row::new()
            .push(label)
            .push(dots)
            .spacing(spacing::GAP_12)
            .align_y(alignment::Vertical::Center)
            .into()
    }
}

struct DotsWidget<'a, Message> {
    count: usize,
    active: usize,
    on_click: Option<Box<dyn Fn(usize) -> Message + 'a>>,
}

#[derive(Default)]
struct DotsState {
    tracks: Vec<Hover>,
    last_active: usize,
    last_count: usize,
    pressed: Option<usize>,
}

fn lerp(from: f32, to: f32, t: f32) -> f32 {
    from + (to - from) * t
}

fn total_width(count: usize) -> f32 {
    if count == 0 {
        return 0.0;
    }
    ACTIVE_WIDTH + (count as f32 - 1.0) * (DOT_DIAMETER + DOT_GAP)
}

fn sync_state(state: &mut DotsState, count: usize, active: usize) {
    if state.last_count != count {
        state.tracks = (0..count)
            .map(|i| Hover::settled(i == active).with_fade(PILL_FADE))
            .collect();
        state.last_active = active;
        state.last_count = count;
        state.pressed = None;
        return;
    }
    if state.last_active != active {
        let now = Instant::now();
        let prev = state.last_active;
        // Only the previously-active and newly-active dots animate.
        // Any straggler still mid-flight from an earlier chained click
        // is snapped to 0 so the sum of per-dot factors stays at 1.0
        // and the rendered width matches `total_width`.
        for (i, track) in state.tracks.iter_mut().enumerate() {
            if i == active {
                track.flip(true, now);
            } else if i == prev {
                track.flip(false, now);
            } else {
                *track = Hover::settled(false).with_fade(PILL_FADE);
            }
        }
        state.last_active = active;
    }
}

fn dot_at(state: &DotsState, bounds: Rectangle, point: Point) -> Option<usize> {
    // Hit-test against the rest layout, not the live animating widths.
    // A press and its release can land 240ms apart and the dots' bounds
    // shift the whole time. Pinning to the settled positions keeps each
    // press-release pair anchored to the dot the user aimed at.
    let mut x = bounds.x;
    for i in 0..state.tracks.len() {
        let w = if i == state.last_active {
            ACTIVE_WIDTH
        } else {
            DOT_DIAMETER
        };
        let dot = Rectangle::new(Point::new(x, bounds.y), Size::new(w, DOT_DIAMETER));
        if dot.contains(point) {
            return Some(i);
        }
        x += w + DOT_GAP;
    }
    None
}

impl<'a, Message> Widget<Message, Theme, IcedRenderer> for DotsWidget<'a, Message>
where
    Message: 'a,
{
    fn size(&self) -> Size<Length> {
        Size {
            width: Length::Fixed(total_width(self.count)),
            height: Length::Fixed(DOT_DIAMETER),
        }
    }

    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<DotsState>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(DotsState::default())
    }

    fn layout(
        &mut self,
        tree: &mut Tree,
        _renderer: &IcedRenderer,
        _limits: &layout::Limits,
    ) -> layout::Node {
        let state = tree.state.downcast_mut::<DotsState>();
        sync_state(state, self.count, self.active);
        layout::Node::new(Size::new(total_width(self.count), DOT_DIAMETER))
    }

    fn draw(
        &self,
        tree: &Tree,
        renderer: &mut IcedRenderer,
        _theme: &Theme,
        _style: &renderer::Style,
        layout: Layout<'_>,
        _cursor: mouse::Cursor,
        _viewport: &Rectangle,
    ) {
        let state = tree.state.downcast_ref::<DotsState>();
        let bounds = layout.bounds();
        let now = Instant::now();

        let inactive_color =
            color::with_alpha(color::WHITE, color::srgb_alpha(INACTIVE_ALPHA));
        let active_color = color::primary();

        let mut x = bounds.x;
        for track in &state.tracks {
            let factor = track.current(now);
            let w = lerp(DOT_DIAMETER, ACTIVE_WIDTH, factor);
            let fill = color::mix(inactive_color, active_color, factor);

            renderer.fill_quad(
                Quad {
                    bounds: Rectangle::new(
                        Point::new(x, bounds.y),
                        Size::new(w, DOT_DIAMETER),
                    ),
                    border: Border {
                        radius: PILL_RADIUS.into(),
                        ..Border::default()
                    },
                    ..Quad::default()
                },
                fill,
            );

            x += w + DOT_GAP;
        }
    }

    fn update(
        &mut self,
        tree: &mut Tree,
        event: &Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        _renderer: &IcedRenderer,
        _clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
        _viewport: &Rectangle,
    ) {
        let state = tree.state.downcast_mut::<DotsState>();
        sync_state(state, self.count, self.active);

        // Drive the redraw chain off any event, not only RedrawRequested.
        // A flip caught inside `layout()` has no `shell` to pump itself,
        // so the next event that lands here picks up the slack.
        let now = Instant::now();
        if state.tracks.iter().any(|t| t.animating(now)) {
            shell.request_redraw();
        }

        let bounds = layout.bounds();

        if matches!(event, Event::Window(window::Event::RedrawRequested(_))) {
            return;
        }

        if self.on_click.is_none() {
            return;
        }

        match event {
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) => {
                if let Some(point) = cursor.position()
                    && let Some(i) = dot_at(state, bounds, point)
                {
                    state.pressed = Some(i);
                    shell.capture_event();
                }
            }
            Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)) => {
                let pressed = state.pressed.take();
                let Some(i) = pressed else {
                    return;
                };
                let Some(point) = cursor.position() else {
                    return;
                };
                if dot_at(state, bounds, point) != Some(i) {
                    return;
                }
                if let Some(on_click) = self.on_click.as_ref() {
                    shell.publish(on_click(i));
                }
                shell.capture_event();
            }
            _ => {}
        }
    }

    fn mouse_interaction(
        &self,
        tree: &Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        _viewport: &Rectangle,
        _renderer: &IcedRenderer,
    ) -> mouse::Interaction {
        if self.on_click.is_none() {
            return mouse::Interaction::None;
        }

        let state = tree.state.downcast_ref::<DotsState>();
        let bounds = layout.bounds();

        if let Some(point) = cursor.position()
            && dot_at(state, bounds, point).is_some()
        {
            mouse::Interaction::Pointer
        } else {
            mouse::Interaction::None
        }
    }
}
