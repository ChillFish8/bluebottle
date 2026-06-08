//! Poster Fan. Three posters share a fixed box, pinned to the right edge
//! and fanning leftward. The lead sits flush and full-size with a bottom
//! gradient to seat any overlay text. Each step back tucks in by a fixed
//! offset and shrinks slightly, with a Gaussian-blurred copy of the
//! artwork plus a deepening ink so the deck reads as solid cards
//! receding rather than translucent ghosts. Geometry constants live at
//! the top of the file.
//!
//! Clicking a poster promotes it to the front. The previously focused
//! poster slides to the back, and the remaining one takes the now-vacant
//! middle slot. The widget reports the chosen poster's input index through
//! `on_click` so the consumer can route to the underlying detail. The
//! reorder transitions across 380ms with an emphasized decelerate curve so
//! position, scale, and tint all settle together. The z-order of the
//! falling and rising cards, and the blur/tint they wear, all swap at the
//! midpoint of the transition so they cross paths naturally without one
//! popping over the other.
//!
//! The blur runs through the public `blurred_image` builder, so the GPU
//! pipeline is shared with every other frosted surface in the app. Each
//! input poster owns one shader child with a fixed blur radius so the
//! GPU cache holds one entry per backdrop instead of churning per frame.
//! The lead position bypasses its shader and renders a sharp image handle
//! on top, derived lazily from the backdrop bytes and cached in tree
//! state so a per-frame `view()` rebuild does not re-clone the pixels.

use std::time::{Duration, Instant};

use iced::advanced::renderer::Quad;
use iced::advanced::widget::{Operation, Tree, tree};
use iced::advanced::{Clipboard, Layout, Renderer, Shell, Widget, layout, renderer};
use iced::gradient::Linear;
use iced::widget::image;
use iced::{
    Background,
    Border,
    Color,
    Element,
    Event,
    Length,
    Point,
    Radians,
    Rectangle,
    Renderer as IcedRenderer,
    Shadow,
    Size,
    Theme,
    mouse,
    window,
};
use std::f32::consts::PI;

use crate::widget::blur::Backdrop;
use crate::widget::blurred_image::{BlurRegion, blurred_image};
use crate::{border, color, easing, style};

const FAN_W: f32 = 392.0;
const FAN_H: f32 = 336.0;

const POSTER_W: f32 = 224.0;
const POSTER_H: f32 = 336.0;
const POSTER_RADIUS: f32 = border::ROUNDED_3XL;

const DEPTHS: usize = 3;
const _: () = assert!(
    DEPTHS == 3,
    "promote() encodes the three-card permutations directly"
);

/// Where in the eased transition the rising and falling cards swap z-order,
/// blur, and tint. At this factor both cards are roughly the same scale and
/// overlap most, so the swap reads as a natural cross rather than a pop.
const Z_SWAP_FACTOR: f32 = 0.5;

const STEP_SCALE: f32 = 0.08;

const TRANSLATE_PER_DEPTH: [f32; DEPTHS] = [0.0, 56.0, 112.0];

const BACK_TINT_STEP: f32 = 0.40;

const LEAD_GRADIENT_FROM: f32 = 0.58;
const LEAD_GRADIENT_BLACK_ALPHA: f32 = 0.55;

const TRANSITION: Duration = Duration::from_millis(380);

/// Gaussian blur radius applied to every back poster's shader child.
/// Fixed at construction so the blur pipeline keeps a single cache entry
/// per backdrop. The lead position skips the shader entirely.
const BLUR_RADIUS: f32 = 8.0;

/// Drop shadow under every poster in the deck. Keeps the cards reading as
/// physical objects layered above the backdrop. Authored as one of the
/// shared `style::ELEVATION_*` recipes so changes propagate everywhere.
const SHADOW: Shadow = style::ELEVATION_INLINE;

/// Margin around each poster's layer so the shadow's blur and downward
/// offset are not clipped by the surrounding `with_layer` bounds. Sized
/// to comfortably cover [`SHADOW`]'s offset plus blur radius.
const SHADOW_MARGIN: f32 = 24.0;

/// Builds a Poster Fan from three poster backdrops laid front to back.
pub fn poster_fan<'a, Message>(
    posters: [Backdrop; DEPTHS],
) -> PosterFan<'a, Message> {
    PosterFan {
        posters,
        on_click: None,
    }
}

/// Builder for [`poster_fan`].
pub struct PosterFan<'a, Message> {
    posters: [Backdrop; DEPTHS],
    on_click: Option<Box<dyn Fn(usize) -> Message + 'a>>,
}

impl<'a, Message> PosterFan<'a, Message> {
    /// Sets the click handler. The callback receives the input index of
    /// the chosen poster.
    pub fn on_click(mut self, on_click: impl Fn(usize) -> Message + 'a) -> Self {
        self.on_click = Some(Box::new(on_click));
        self
    }
}

impl<'a, Message> From<PosterFan<'a, Message>> for Element<'a, Message>
where
    Message: 'a,
{
    fn from(fan: PosterFan<'a, Message>) -> Self {
        let children: Vec<Element<'a, Message>> = fan
            .posters
            .iter()
            .cloned()
            .map(make_shader_child)
            .collect();
        Element::new(PosterFanWidget {
            posters: fan.posters,
            children,
            on_click: fan.on_click,
        })
    }
}

fn make_shader_child<'a, Message>(backdrop: Backdrop) -> Element<'a, Message>
where
    Message: 'a,
{
    blurred_image(backdrop)
        .blur(BLUR_RADIUS)
        .corner_radius(POSTER_RADIUS)
        .regions_fn(|size: Size| {
            vec![BlurRegion::rounded(
                Rectangle::new(Point::ORIGIN, size),
                POSTER_RADIUS,
            )]
        })
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}

fn backdrop_to_handle(backdrop: &Backdrop) -> image::Handle {
    image::Handle::from_rgba(
        backdrop.width(),
        backdrop.height(),
        backdrop.rgba().to_vec(),
    )
}

struct PosterFanWidget<'a, Message> {
    posters: [Backdrop; DEPTHS],
    children: Vec<Element<'a, Message>>,
    on_click: Option<Box<dyn Fn(usize) -> Message + 'a>>,
}

struct FanState {
    order: [usize; DEPTHS],
    transition: Option<Transition>,
    pressed: Option<usize>,
    snapshot: Snapshot,
    handles: Option<[image::Handle; DEPTHS]>,
    backdrop_keys: [usize; DEPTHS],
}

/// One frame's resolved geometry. `layout()` computes this once per call
/// and `draw()` reads it back, so the two paths never disagree about which
/// side of [`Z_SWAP_FACTOR`] they are on.
#[derive(Clone, Copy)]
struct Snapshot {
    from_order: [usize; DEPTHS],
    to_order: [usize; DEPTHS],
    render_order: [usize; DEPTHS],
    t: f32,
}

const REST_SNAPSHOT: Snapshot = Snapshot {
    from_order: [0, 1, 2],
    to_order: [0, 1, 2],
    render_order: [0, 1, 2],
    t: 1.0,
};

impl Default for FanState {
    fn default() -> Self {
        Self {
            order: [0, 1, 2],
            transition: None,
            pressed: None,
            snapshot: REST_SNAPSHOT,
            handles: None,
            backdrop_keys: [0; DEPTHS],
        }
    }
}

#[derive(Clone, Copy)]
struct Transition {
    from: [usize; DEPTHS],
    to: [usize; DEPTHS],
    started: Instant,
}

impl Transition {
    fn factor(&self, now: Instant) -> f32 {
        let raw = (now.duration_since(self.started).as_secs_f32()
            / TRANSITION.as_secs_f32())
        .clamp(0.0, 1.0);
        easing::EMPHASIZED_DECELERATE.y_at_x(raw)
    }

    fn done(&self, now: Instant) -> bool {
        now.duration_since(self.started) >= TRANSITION
    }
}

fn poster_rect(depth: usize, bounds: Rectangle) -> Rectangle {
    let scale = 1.0 - STEP_SCALE * depth as f32;
    let w = POSTER_W * scale;
    let h = POSTER_H * scale;
    let right_edge = FAN_W - TRANSLATE_PER_DEPTH[depth];
    let x = right_edge - w;
    let y = (FAN_H - h) * 0.5;
    Rectangle::new(Point::new(bounds.x + x, bounds.y + y), Size::new(w, h))
}

fn back_tint_alpha(depth: usize) -> f32 {
    BACK_TINT_STEP * depth as f32
}

fn lerp(from: f32, to: f32, t: f32) -> f32 {
    from + (to - from) * t
}

fn lerp_rect(from: Rectangle, to: Rectangle, t: f32) -> Rectangle {
    Rectangle::new(
        Point::new(lerp(from.x, to.x, t), lerp(from.y, to.y, t)),
        Size::new(
            lerp(from.width, to.width, t),
            lerp(from.height, to.height, t),
        ),
    )
}

fn depths_of(order: &[usize; DEPTHS]) -> [usize; DEPTHS] {
    let mut depths = [0; DEPTHS];
    for (depth, &input) in order.iter().enumerate() {
        depths[input] = depth;
    }
    depths
}

/// Promotes the poster at `clicked_depth` to the front. The poster
/// previously at the front slides to the back. Any remaining poster takes
/// the now-vacant middle slot. No-op when the front is clicked.
fn promote(order: [usize; DEPTHS], clicked_depth: usize) -> [usize; DEPTHS] {
    match clicked_depth {
        1 => [order[1], order[2], order[0]],
        2 => [order[2], order[1], order[0]],
        _ => order,
    }
}

fn hit_depth(bounds: Rectangle, point: Point) -> Option<usize> {
    (0..DEPTHS).find(|&depth| poster_rect(depth, bounds).contains(point))
}

/// Inflates `rect` by [`SHADOW_MARGIN`] on every side. The outer
/// `with_layer` uses this so the drop shadow's blur and offset are not
/// clipped at the poster's edge.
fn shadow_layer_bounds(rect: Rectangle) -> Rectangle {
    Rectangle {
        x: rect.x - SHADOW_MARGIN,
        y: rect.y - SHADOW_MARGIN,
        width: rect.width + 2.0 * SHADOW_MARGIN,
        height: rect.height + 2.0 * SHADOW_MARGIN,
    }
}

/// Resolves the current orders and eased factor and stores them on the
/// shared [`Snapshot`]. Calling this in `layout()` makes the resulting
/// child positions the single source of truth for `draw()`, so the two
/// passes can never land on different sides of [`Z_SWAP_FACTOR`].
fn refresh_snapshot(state: &mut FanState, now: Instant) {
    let (from_order, to_order, t) = match state.transition {
        Some(transition) if !transition.done(now) => (
            transition.from,
            transition.to,
            transition.factor(now),
        ),
        _ => (state.order, state.order, 1.0),
    };
    let render_order = if t < Z_SWAP_FACTOR {
        from_order
    } else {
        to_order
    };
    state.snapshot = Snapshot {
        from_order,
        to_order,
        render_order,
        t,
    };
}

/// Lazily derives an `image::Handle` per backdrop so the sharp lead
/// rendering does not pay for a 4-byte-per-pixel clone on every
/// `view()` rebuild. The cache invalidates when any backdrop's
/// `Arc`-identity changes.
fn ensure_handles(state: &mut FanState, posters: &[Backdrop; DEPTHS]) {
    let current_keys: [usize; DEPTHS] =
        std::array::from_fn(|i| posters[i].key());
    if state.handles.is_none() || state.backdrop_keys != current_keys {
        state.handles = Some(std::array::from_fn(|i| backdrop_to_handle(&posters[i])));
        state.backdrop_keys = current_keys;
    }
}

impl<'a, Message> Widget<Message, Theme, IcedRenderer> for PosterFanWidget<'a, Message>
where
    Message: 'a,
{
    fn size(&self) -> Size<Length> {
        Size {
            width: Length::Fixed(FAN_W),
            height: Length::Fixed(FAN_H),
        }
    }

    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<FanState>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(FanState::default())
    }

    fn children(&self) -> Vec<Tree> {
        self.children.iter().map(Tree::new).collect()
    }

    fn diff(&self, tree: &mut Tree) {
        tree.diff_children(&self.children);
    }

    fn layout(
        &mut self,
        tree: &mut Tree,
        renderer: &IcedRenderer,
        _limits: &layout::Limits,
    ) -> layout::Node {
        let state = tree.state.downcast_mut::<FanState>();
        ensure_handles(state, &self.posters);
        refresh_snapshot(state, Instant::now());
        let snapshot = state.snapshot;

        let from_depths = depths_of(&snapshot.from_order);
        let to_depths = depths_of(&snapshot.to_order);
        let bounds = Rectangle::new(Point::ORIGIN, Size::new(FAN_W, FAN_H));

        let mut child_nodes = Vec::with_capacity(DEPTHS);
        for input in 0..DEPTHS {
            let rect = lerp_rect(
                poster_rect(from_depths[input], bounds),
                poster_rect(to_depths[input], bounds),
                snapshot.t,
            );
            let limits = layout::Limits::new(
                Size::new(rect.width, rect.height),
                Size::new(rect.width, rect.height),
            );
            let node = self.children[input].as_widget_mut().layout(
                &mut tree.children[input],
                renderer,
                &limits,
            );
            child_nodes.push(node.move_to(Point::new(rect.x, rect.y)));
        }

        layout::Node::with_children(Size::new(FAN_W, FAN_H), child_nodes)
    }

    fn draw(
        &self,
        tree: &Tree,
        renderer: &mut IcedRenderer,
        theme: &Theme,
        style: &renderer::Style,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
    ) {
        use iced::advanced::image::Renderer as ImageRenderer;

        let state = tree.state.downcast_ref::<FanState>();
        let snapshot = state.snapshot;
        let handles = state
            .handles
            .as_ref()
            .expect("layout() populates handles before draw()");
        let child_layouts: Vec<Layout<'_>> = layout.children().collect();

        for depth_idx in (0..DEPTHS).rev() {
            let input = snapshot.render_order[depth_idx];
            let child_layout = child_layouts[input];
            let rect = child_layout.bounds();
            // Match the shader's fixed-pixel corner so the tint and the
            // image edges line up. The shader's `corner_radius` is set at
            // construction and cannot scale per frame.
            let rounded = Border {
                radius: POSTER_RADIUS.into(),
                ..Border::default()
            };

            // iced_wgpu batches every quad before every image inside a layer,
            // so the tint and gradient land in their own sublayers to paint on
            // top of the artwork. The outer layer is inflated by
            // SHADOW_MARGIN so the drop shadow has room to breathe.
            renderer.with_layer(shadow_layer_bounds(rect), |renderer| {
                renderer.fill_quad(
                    Quad {
                        bounds: rect,
                        border: rounded,
                        shadow: SHADOW,
                        ..Quad::default()
                    },
                    Color::TRANSPARENT,
                );

                if depth_idx == 0 {
                    ImageRenderer::draw_image(
                        renderer,
                        iced::advanced::image::Image {
                            handle: handles[input].clone(),
                            filter_method: image::FilterMethod::Linear,
                            rotation: Radians(0.0),
                            border_radius: POSTER_RADIUS.into(),
                            opacity: 1.0,
                            snap: true,
                        },
                        rect,
                        rect,
                    );
                } else {
                    self.children[input].as_widget().draw(
                        &tree.children[input],
                        renderer,
                        theme,
                        style,
                        child_layout,
                        cursor,
                        viewport,
                    );
                }

                let tint_alpha = back_tint_alpha(depth_idx);
                if tint_alpha > 0.001 {
                    renderer.with_layer(rect, |renderer| {
                        renderer.fill_quad(
                            Quad {
                                bounds: rect,
                                border: rounded,
                                ..Quad::default()
                            },
                            color::with_alpha(color::STACK_INK, tint_alpha),
                        );
                    });
                }

                if depth_idx == 0 {
                    let gradient = Linear::new(Radians(PI))
                        .add_stop(0.0, Color::TRANSPARENT)
                        .add_stop(LEAD_GRADIENT_FROM, Color::TRANSPARENT)
                        .add_stop(
                            1.0,
                            color::with_alpha(Color::BLACK, LEAD_GRADIENT_BLACK_ALPHA),
                        );
                    renderer.with_layer(rect, |renderer| {
                        renderer.fill_quad(
                            Quad {
                                bounds: rect,
                                border: rounded,
                                ..Quad::default()
                            },
                            Background::Gradient(gradient.into()),
                        );
                    });
                }
            });
        }
    }

    fn operate(
        &mut self,
        tree: &mut Tree,
        layout: Layout<'_>,
        renderer: &IcedRenderer,
        operation: &mut dyn Operation,
    ) {
        let mut child_layouts = layout.children();
        for (child, child_tree) in
            self.children.iter_mut().zip(tree.children.iter_mut())
        {
            let child_layout = child_layouts.next().expect("child layout");
            child
                .as_widget_mut()
                .operate(child_tree, child_layout, renderer, operation);
        }
    }

    fn update(
        &mut self,
        tree: &mut Tree,
        event: &Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &IcedRenderer,
        clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
        viewport: &Rectangle,
    ) {
        // Forward to shader children first so they keep their per-frame
        // bookkeeping in step with the redraw clock.
        let mut child_layouts = layout.children();
        for (child, child_tree) in
            self.children.iter_mut().zip(tree.children.iter_mut())
        {
            let child_layout = child_layouts.next().expect("child layout");
            child.as_widget_mut().update(
                child_tree,
                event,
                child_layout,
                cursor,
                renderer,
                clipboard,
                shell,
                viewport,
            );
        }

        let bounds = layout.bounds();
        let state = tree.state.downcast_mut::<FanState>();

        if let Event::Window(window::Event::RedrawRequested(_)) = event {
            // Use Instant::now() consistently with layout() so the
            // completion check and the eased factor never disagree
            // about which side of the deadline we are on.
            let now = Instant::now();
            if let Some(transition) = state.transition {
                if transition.done(now) {
                    state.transition = None;
                } else {
                    shell.request_redraw();
                }
            }
            return;
        }

        if self.on_click.is_none() || state.transition.is_some() {
            return;
        }

        match event {
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) => {
                if let Some(point) = cursor.position()
                    && let Some(depth) = hit_depth(bounds, point)
                {
                    state.pressed = Some(depth);
                    shell.capture_event();
                }
            }
            Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)) => {
                let pressed = state.pressed.take();
                let Some(pressed_depth) = pressed else {
                    return;
                };
                let Some(point) = cursor.position() else {
                    return;
                };
                if hit_depth(bounds, point) != Some(pressed_depth) {
                    return;
                }

                let chosen_input = state.order[pressed_depth];
                if let Some(on_click) = self.on_click.as_ref() {
                    shell.publish(on_click(chosen_input));
                }
                shell.capture_event();

                if pressed_depth == 0 {
                    return;
                }

                let new_order = promote(state.order, pressed_depth);
                state.transition = Some(Transition {
                    from: state.order,
                    to: new_order,
                    started: Instant::now(),
                });
                state.order = new_order;
                shell.request_redraw();
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

        // Clicks are dropped while a transition is in flight, so the
        // cursor should not advertise the deck as pressable in that
        // window.
        let state = tree.state.downcast_ref::<FanState>();
        if state.transition.is_some() {
            return mouse::Interaction::None;
        }

        let bounds = layout.bounds();
        if let Some(point) = cursor.position()
            && hit_depth(bounds, point).is_some()
        {
            mouse::Interaction::Pointer
        } else {
            mouse::Interaction::None
        }
    }
}
