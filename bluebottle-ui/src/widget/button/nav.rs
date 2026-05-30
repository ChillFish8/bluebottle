use std::f32::consts::{FRAC_PI_2, PI};
use std::time::{Duration, Instant};

use iced::advanced::renderer::{Quad, Style as RendererStyle};
use iced::advanced::widget::{Operation, Tree, tree};
use iced::advanced::{Clipboard, Layout, Renderer, Shell, Widget, layout};
use iced::widget::{canvas, column, container};
use iced::{
    Border,
    Center,
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

use crate::animate::hover::{EPSILON, Hover, PressState};
use crate::{color, font, icon, text};

const NAV_ICON_PADDING: [u16; 2] = [4, 16];
const NAV_PUCK_WIDTH: f32 = 44.0;
const NAV_PUCK_HEIGHT: f32 = 28.0;
// r999 in the spec. A radius past half the height renders a full pill.
const NAV_PUCK_RADIUS: f32 = 999.0;
// The selection border and the colour and weight shift run slower than the
// shared hover fade so the wrap reads as a deliberate transition.
const NAV_SELECT_FADE: Duration = Duration::from_millis(175);

/// A navbar button.
///
/// Icon over label, vertically centred. The pill behind the icon animates
/// both on hover (cursor enter and leave) and when `selected` toggles. The
/// content scales down briefly on press. When `selected` is true the press
/// dispatches no message so reselecting the active entry is a no-op.
pub fn nav<'a, Message>(
    label: &'a str,
    icon: &'a str,
    selected: bool,
    message: Message,
) -> Element<'a, Message>
where
    Message: Clone + 'a,
{
    Element::new(NavButton::new(label, icon, selected, message))
}

struct NavButton<'a, Message> {
    selected: bool,
    message: Message,
    content: Element<'a, Message>,
}

impl<'a, Message> NavButton<'a, Message>
where
    Message: Clone + 'a,
{
    fn new(
        label: &'a str,
        icon_name: &'a str,
        selected: bool,
        message: Message,
    ) -> Self {
        let icon_text = icon::filled(icon_name).size(20);
        let label_text = text::micro_label(label).align_x(Center);
        let label_text = if selected {
            label_text.font(font::semibold())
        } else {
            label_text
        };

        // Built once and owned so the widget tree state stays consistent
        // across frames. Rebuilding each call would hand `diff_children` a
        // fresh Element every frame and lose the animated state.
        let content =
            column![container(icon_text).padding(NAV_ICON_PADDING), label_text]
                .align_x(Center)
                .into();

        Self {
            selected,
            message,
            content,
        }
    }
}

#[derive(Clone, Copy, Default)]
struct NavState {
    press: PressState,
    selected: Hover,
}

fn puck_bounds(content_layout: Layout<'_>) -> Option<Rectangle> {
    let icon = content_layout.children().next()?.bounds();
    Some(Rectangle {
        x: icon.center_x() - NAV_PUCK_WIDTH / 2.0,
        y: icon.center_y() - NAV_PUCK_HEIGHT / 2.0,
        width: NAV_PUCK_WIDTH,
        height: NAV_PUCK_HEIGHT,
    })
}

/// Straight segments used to approximate each end-cap arc. Enough to read as
/// smooth at the puck's 14px corner radius.
const NAV_BORDER_ARC_STEPS: usize = 24;

/// Strokes the partial puck outline. The accent line is drawn at a constant
/// opacity and its length tracks `factor`, so selecting and deselecting wraps
/// the border on and off from the bottom centre rather than fading it.
fn draw_selected_border(
    renderer: &mut iced::Renderer,
    bounds: Rectangle,
    puck: Rectangle,
    factor: f32,
) {
    use iced::advanced::graphics::geometry::Renderer as _;

    // The puck top sits flush with the widget top, so the stroke is built in a
    // frame padded on every side. Without the margin the 1px line's outer half
    // falls outside the frame and the top of the wrap is clipped.
    const MARGIN: f32 = 2.0;

    // Work in padded widget-local coordinates, then translate back out.
    let puck = Rectangle {
        x: puck.x - bounds.x + MARGIN,
        y: puck.y - bounds.y + MARGIN,
        width: puck.width,
        height: puck.height,
    };

    let mut builder = canvas::path::Builder::new();
    for dir in [1.0_f32, -1.0] {
        trace_partial(&mut builder, &half_outline(puck, dir), factor);
    }

    let frame_size =
        Size::new(bounds.width + MARGIN * 2.0, bounds.height + MARGIN * 2.0);
    let mut frame = canvas::Frame::new(renderer, frame_size);
    frame.stroke(
        &builder.build(),
        canvas::Stroke::default()
            .with_color(color::with_alpha(color::primary(), color::srgb_alpha(0.55)))
            .with_width(1.0)
            .with_line_cap(canvas::LineCap::Round)
            .with_line_join(canvas::LineJoin::Round),
    );
    let geometry = frame.into_geometry();

    let origin = Vector::new(bounds.x - MARGIN, bounds.y - MARGIN);
    renderer.with_translation(origin, |renderer| {
        renderer.draw_geometry(geometry);
    });
}

/// Builds the route for one half of the puck outline, from the bottom centre up
/// and around to the top centre. `dir` is `1.0` for the right half and `-1.0`
/// for the mirrored left half, so the two halves grow symmetrically.
fn half_outline(puck: Rectangle, dir: f32) -> Vec<Point> {
    let r = puck.height / 2.0;
    let cx = puck.x + puck.width / 2.0;
    let cy = puck.y + puck.height / 2.0;
    let arc_cx = cx + dir * (puck.width / 2.0 - r).max(0.0);

    let mut points = Vec::with_capacity(NAV_BORDER_ARC_STEPS + 3);
    points.push(Point::new(cx, cy + r));
    points.push(Point::new(arc_cx, cy + r));

    // Sweep the end-cap from the bottom round to the top, bulging outward.
    for step in 1..=NAV_BORDER_ARC_STEPS {
        let theta = FRAC_PI_2 - PI * (step as f32 / NAV_BORDER_ARC_STEPS as f32);
        points.push(Point::new(
            arc_cx + dir * r * theta.cos(),
            cy + r * theta.sin(),
        ));
    }

    points.push(Point::new(cx, cy - r));
    points
}

/// Adds the leading `factor` fraction of `route`'s arc length to `builder` as a
/// fresh sub-path, interpolating the final segment so the tip lands mid-edge.
fn trace_partial(builder: &mut canvas::path::Builder, route: &[Point], factor: f32) {
    let total: f32 = route.windows(2).map(|w| distance(w[0], w[1])).sum();
    let target = factor.clamp(0.0, 1.0) * total;

    builder.move_to(route[0]);

    let mut walked = 0.0;
    for window in route.windows(2) {
        let segment = distance(window[0], window[1]);
        if walked + segment >= target {
            let t = if segment > 0.0 {
                (target - walked) / segment
            } else {
                0.0
            };
            builder.line_to(lerp(window[0], window[1], t));
            break;
        }
        builder.line_to(window[1]);
        walked += segment;
    }
}

fn distance(a: Point, b: Point) -> f32 {
    (a.x - b.x).hypot(a.y - b.y)
}

fn lerp(a: Point, b: Point, t: f32) -> Point {
    Point::new(a.x + (b.x - a.x) * t, a.y + (b.y - a.y) * t)
}

impl<'a, Message> Widget<Message, iced::Theme, iced::Renderer> for NavButton<'a, Message>
where
    Message: Clone + 'a,
{
    fn size(&self) -> Size<Length> {
        Size {
            width: Length::Shrink,
            height: Length::Shrink,
        }
    }

    fn layout(
        &mut self,
        tree: &mut Tree,
        renderer: &iced::Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        let node = self.content.as_widget_mut().layout(
            tree.children.first_mut().expect("nav child tree"),
            renderer,
            limits,
        );
        layout::Node::with_children(node.size(), vec![node])
    }

    fn draw(
        &self,
        tree: &Tree,
        renderer: &mut iced::Renderer,
        theme: &iced::Theme,
        _style: &RendererStyle,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
    ) {
        let state = tree.state.downcast_ref::<NavState>();
        let now = Instant::now();
        let hover_factor = state.press.hover.current(now);
        let selected_factor = state.selected.current(now);

        let pill_factor = hover_factor.max(selected_factor);
        let content_layout = layout.children().next().expect("nav child layout");
        let puck = puck_bounds(content_layout);

        if let Some(puck) = puck
            && pill_factor > EPSILON
        {
            // Glass tint. Hover settles at 5% white, selected lifts it to 6%.
            // The opacities are authored in sRGB so they go through
            // `srgb_alpha` to stay as faint as they read in the design.
            let fill_alpha = (color::srgb_alpha(0.05) * hover_factor)
                .max(color::srgb_alpha(0.06) * selected_factor);

            renderer.fill_quad(
                Quad {
                    bounds: puck,
                    border: Border {
                        radius: NAV_PUCK_RADIUS.into(),
                        ..Border::default()
                    },
                    ..Quad::default()
                },
                color::with_alpha(color::WHITE, fill_alpha),
            );
        }

        // The accent border wraps up from the puck's bottom centre as the entry
        // activates and unwinds back down when it deselects.
        if let Some(puck) = puck
            && selected_factor > EPSILON
        {
            draw_selected_border(renderer, layout.bounds(), puck, selected_factor);
        }

        let content_style = RendererStyle {
            text_color: color::ease(
                color::TEXT_PRIMARY,
                color::primary(),
                selected_factor,
            ),
        };

        self.content.as_widget().draw(
            tree.children.first().expect("nav child tree"),
            renderer,
            theme,
            &content_style,
            content_layout,
            cursor,
            viewport,
        );
    }

    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<NavState>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(NavState {
            selected: Hover::settled(self.selected).with_fade(NAV_SELECT_FADE),
            ..NavState::default()
        })
    }

    fn children(&self) -> Vec<Tree> {
        vec![Tree::new(&self.content)]
    }

    fn diff(&self, tree: &mut Tree) {
        tree.diff_children(std::slice::from_ref(&self.content));

        let state = tree.state.downcast_mut::<NavState>();
        state.selected.flip(self.selected, Instant::now());
    }

    fn operate(
        &mut self,
        tree: &mut Tree,
        layout: Layout<'_>,
        renderer: &iced::Renderer,
        operation: &mut dyn Operation,
    ) {
        let content_layout = layout.children().next().expect("nav child layout");
        self.content.as_widget_mut().operate(
            tree.children.first_mut().expect("nav child tree"),
            content_layout,
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
        let content_layout = layout.children().next().expect("nav child layout");
        self.content.as_widget_mut().update(
            tree.children.first_mut().expect("nav child tree"),
            event,
            content_layout,
            cursor,
            renderer,
            clipboard,
            shell,
            viewport,
        );

        let now = Instant::now();
        let bounds = layout.bounds();
        let over = cursor.is_over(bounds);
        let state = tree.state.downcast_mut::<NavState>();

        match event {
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left))
                if !self.selected =>
            {
                if !shell.is_event_captured() {
                    state.press.press(over);
                }
            },

            Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)) => {
                // Always clear any in-flight press cycle, even when this
                // entry is now selected, so a selected flip mid-press
                // cannot leak `pressed = true` into a later click.
                let dispatch = state.press.release(over);
                if dispatch && !self.selected && !shell.is_event_captured() {
                    shell.publish(self.message.clone());
                    shell.capture_event();
                }
            },

            _ => {
                if !self.selected && state.press.reconcile(over, now) {
                    shell.request_redraw();
                }
                if let Event::Window(window::Event::RedrawRequested(_)) = event
                    && (state.press.animating(now) || state.selected.animating(now))
                {
                    shell.request_redraw();
                }
            },
        }
    }

    fn mouse_interaction(
        &self,
        tree: &Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
        renderer: &iced::Renderer,
    ) -> mouse::Interaction {
        let content_layout = layout.children().next().expect("nav child layout");
        let inner = self.content.as_widget().mouse_interaction(
            tree.children.first().expect("nav child tree"),
            content_layout,
            cursor,
            viewport,
            renderer,
        );
        if !matches!(inner, mouse::Interaction::None | mouse::Interaction::Idle) {
            return inner;
        }

        if !self.selected && cursor.is_over(layout.bounds()) {
            return mouse::Interaction::Pointer;
        }

        mouse::Interaction::None
    }
}
