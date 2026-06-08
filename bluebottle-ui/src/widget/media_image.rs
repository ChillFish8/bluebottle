//! A frosted-glass media image. Wraps `blurred_image` so corner pills and
//! the optional centre overlay frost the source behind them in one shader
//! pass, with the same hover affordances `media_card`-style cards expect.
//!
//! With no `on_press` set the image is inert. Pills still draw at rest, the
//! centre overlay still draws at rest, but the shadow, hover dim, animated
//! border, and centre scale-in stay dormant. With `on_press` set, hovering
//! animates a drop shadow under the image, a tint over it, and scales the
//! centre overlay in from the middle. The primary border around the image
//! is animated as well by default and can be turned off independently with
//! `.border(false)`.

use std::sync::{Arc, Mutex};
use std::time::Instant;

use iced::advanced::renderer::{Quad, Style};
use iced::advanced::widget::{Operation, Tree, tree};
use iced::advanced::{Clipboard, Layout, Renderer, Shell, Widget, layout};
use iced::{
    Border,
    Color,
    Element,
    Event,
    Length,
    Point,
    Rectangle,
    Size,
    Transformation,
    mouse,
    window,
};

use crate::animate::hover::{EPSILON, Hover};
use crate::widget::blur::Backdrop;
use crate::widget::blurred_image;
use crate::{border, color, spacing, style};

/// Background-tint alpha applied over the image at full hover.
const TINT_ALPHA: f32 = 0.75;

/// Spacing between consecutive pills that share a corner. Each pill keeps
/// Where a pill anchors on the image. Variant order matches the indices
/// the layout uses when bookkeeping per-corner stack offsets.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PillCorner {
    TopLeft = 0,
    TopRight = 1,
    BottomLeft = 2,
    BottomRight = 3,
}

/// Creates a media image over `backdrop`. Non-interactive by default. Set
/// `.on_press(...)` to enable the hover affordances. Add `.pill(corner, ...)`
/// to anchor frosted pills at the image corners, or `.overlay(...)` to layer
/// content at the centre.
pub fn media_image<'a, Message>(backdrop: Backdrop) -> MediaImage<'a, Message>
where
    Message: Clone + 'a,
{
    let intrinsic = Size::new(backdrop.width() as f32, backdrop.height() as f32);
    MediaImage {
        backdrop,
        intrinsic,
        overlay: None,
        pills: Vec::new(),
        on_press: None,
        border: true,
        width: Length::Shrink,
        height: Length::Shrink,
        corner_radius: border::ROUNDED_MD,
        blur_radius: None,
    }
}

/// A configurable media image, built by [`media_image`].
pub struct MediaImage<'a, Message> {
    backdrop: Backdrop,
    intrinsic: Size,
    overlay: Option<Element<'a, Message>>,
    pills: Vec<(PillCorner, Element<'a, Message>)>,
    on_press: Option<Message>,
    border: bool,
    width: Length,
    height: Length,
    corner_radius: f32,
    blur_radius: Option<f32>,
}

impl<'a, Message> MediaImage<'a, Message>
where
    Message: Clone + 'a,
{
    /// Layers `overlay` over the centre of the image. With `on_press` set
    /// the overlay scales in from the centre as the cursor enters and out
    /// as it leaves. Without `on_press` it draws at rest.
    pub fn overlay(mut self, overlay: impl Into<Element<'a, Message>>) -> Self {
        self.overlay = Some(overlay.into());
        self
    }

    /// Pins `content` to one corner of the image with a matching frosted
    /// region behind it. The pill shrink-fits its content and sits at the
    /// shared [`spacing::PAD_12`] from the corner so a `media_image` pill aligns
    /// with the time-left pill on the watch-signal cards.
    pub fn pill(
        mut self,
        corner: PillCorner,
        content: impl Into<Element<'a, Message>>,
    ) -> Self {
        self.pills.push((corner, content.into()));
        self
    }

    /// Sets the press message. Required to enable the hover affordances and
    /// the pointer cursor. Without one the widget is inert.
    pub fn on_press(mut self, message: Message) -> Self {
        self.on_press = Some(message);
        self
    }

    /// Toggles the primary hover border around the image. Defaults to true.
    /// Has no effect when `on_press` is unset.
    pub fn border(mut self, enabled: bool) -> Self {
        self.border = enabled;
        self
    }

    pub fn width(mut self, width: impl Into<Length>) -> Self {
        self.width = width.into();
        self
    }

    pub fn height(mut self, height: impl Into<Length>) -> Self {
        self.height = height.into();
        self
    }

    pub fn corner_radius(mut self, radius: f32) -> Self {
        self.corner_radius = radius;
        self
    }

    /// Overrides the blur radius used for the frosted regions, in source
    /// pixels. Defaults to [`style::IMAGE_BLUR`](crate::style::IMAGE_BLUR).
    pub fn blur(mut self, radius: f32) -> Self {
        self.blur_radius = Some(radius);
        self
    }
}

impl<'a, Message> From<MediaImage<'a, Message>> for Element<'a, Message>
where
    Message: Clone + 'a,
{
    fn from(card: MediaImage<'a, Message>) -> Self {
        // Pill bounds the chrome publishes during layout. The inner
        // blurred_image reads them through `regions_fn` at draw so the
        // frost tracks whatever size each pill shaped to.
        let regions: Arc<Mutex<Vec<blurred_image::BlurRegion>>> =
            Arc::new(Mutex::new(Vec::new()));

        let regions_for_image = Arc::clone(&regions);
        let mut image_builder = blurred_image::blurred_image(card.backdrop)
            .corner_radius(card.corner_radius)
            .width(Length::Fill)
            .height(Length::Fill)
            .regions_fn(move |_| {
                regions_for_image
                    .lock()
                    .map(|guard| guard.clone())
                    .unwrap_or_default()
            });

        if let Some(blur) = card.blur_radius {
            image_builder = image_builder.blur(blur);
        }

        let mut children: Vec<Element<'a, Message>> = Vec::with_capacity(
            1 + card.pills.len() + usize::from(card.overlay.is_some()),
        );
        children.push(image_builder.into());

        let pill_corners: Vec<PillCorner> = card.pills.iter().map(|(c, _)| *c).collect();
        for (_, element) in card.pills {
            children.push(element);
        }

        let overlay_idx = card.overlay.map(|element| {
            children.push(element);
            children.len() - 1
        });

        Element::new(Inner {
            children,
            pill_corners,
            overlay_idx,
            on_press: card.on_press,
            border: card.border,
            corner_radius: card.corner_radius,
            width: card.width,
            height: card.height,
            intrinsic: card.intrinsic,
            shared_regions: regions,
        })
    }
}

/// The image lives at index 0. Pills occupy `1..1 + pill_corners.len()`. The
/// optional centre overlay, when present, sits at `overlay_idx`.
struct Inner<'a, Message> {
    children: Vec<Element<'a, Message>>,
    pill_corners: Vec<PillCorner>,
    overlay_idx: Option<usize>,
    on_press: Option<Message>,
    border: bool,
    corner_radius: f32,
    width: Length,
    height: Length,
    intrinsic: Size,
    shared_regions: Arc<Mutex<Vec<blurred_image::BlurRegion>>>,
}

impl<'a, Message: Clone> Inner<'a, Message> {
    fn interactive(&self) -> bool {
        self.on_press.is_some()
    }

    fn image_bounds(&self, layout: &Layout<'_>) -> Rectangle {
        layout.children().next().expect("image child").bounds()
    }

    /// Whether the centre overlay should receive events. True while the
    /// hover factor is above `EPSILON`, or on the entering frame while the
    /// cursor is over an interactive image. Non-interactive overlays stay
    /// alive at rest so any inner widgets still respond.
    fn overlay_alive(
        &self,
        tree: &Tree,
        layout: &Layout<'_>,
        cursor: mouse::Cursor,
        now: Instant,
    ) -> bool {
        if self.overlay_idx.is_none() {
            return false;
        }
        if !self.interactive() {
            return true;
        }
        let factor = tree.state.downcast_ref::<State>().image.current(now);
        factor > EPSILON || cursor.is_over(self.image_bounds(layout))
    }
}

#[derive(Clone, Copy, Default)]
struct State {
    image: Hover,
    /// Whether a left-button press started over the image. Releases without
    /// a matching press are ignored.
    pressed: bool,
}

impl<'a, Message> Widget<Message, iced::Theme, iced::Renderer> for Inner<'a, Message>
where
    Message: Clone + 'a,
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
        let resolved = limits.resolve(self.width, self.height, self.intrinsic);
        let image_limits = layout::Limits::new(resolved, resolved);
        let image_rect = Rectangle::new(Point::ORIGIN, resolved);

        let mut nodes = Vec::with_capacity(self.children.len());

        let (image, image_tree) = self
            .children
            .iter_mut()
            .zip(tree.children.iter_mut())
            .next()
            .expect("image child");
        let image_node = image
            .as_widget_mut()
            .layout(image_tree, renderer, &image_limits)
            .move_to(Point::ORIGIN);
        nodes.push(image_node);

        // Pills shrink-fit to their content. Pills sharing a corner stack
        // along the corner's edge with a fixed gap so each gets its own
        // frosted region instead of crowding into one rect. The shared
        // region list is republished here so the shader sees the same
        // rects this frame.
        let mut pill_regions = Vec::with_capacity(self.pill_corners.len());
        let pill_limits = layout::Limits::new(Size::ZERO, resolved);
        let mut corner_offsets = [0.0_f32; 4];

        for (offset, &corner) in self.pill_corners.iter().enumerate() {
            let idx = 1 + offset;
            let (child, child_tree) = self
                .children
                .iter_mut()
                .zip(tree.children.iter_mut())
                .nth(idx)
                .expect("pill child");

            let node = child
                .as_widget_mut()
                .layout(child_tree, renderer, &pill_limits);
            let pill_size = node.size();
            let stack_offset = corner_offsets[corner as usize];
            let origin = pill_origin(corner, image_rect, pill_size, stack_offset);

            corner_offsets[corner as usize] += pill_size.width + spacing::GAP_6;

            pill_regions.push(blurred_image::BlurRegion::pill(Rectangle::new(
                origin, pill_size,
            )));
            nodes.push(node.move_to(origin));
        }

        if let Ok(mut guard) = self.shared_regions.lock() {
            *guard = pill_regions;
        }

        if let Some(idx) = self.overlay_idx {
            let overlay_limits = layout::Limits::new(resolved, resolved);
            let (overlay, overlay_tree) = self
                .children
                .iter_mut()
                .zip(tree.children.iter_mut())
                .nth(idx)
                .expect("overlay child");
            let overlay_node = overlay
                .as_widget_mut()
                .layout(overlay_tree, renderer, &overlay_limits)
                .move_to(Point::ORIGIN);
            nodes.push(overlay_node);
        }

        layout::Node::with_children(resolved, nodes)
    }

    fn draw(
        &self,
        tree: &Tree,
        renderer: &mut iced::Renderer,
        theme: &iced::Theme,
        style: &Style,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
    ) {
        let state = tree.state.downcast_ref::<State>();
        let factor = if self.interactive() {
            state.image.current(Instant::now())
        } else {
            0.0
        };
        let draw_hover = factor > EPSILON;

        let mut child_iter = self.children.iter().zip(tree.children.iter());
        let mut layout_iter = layout.children();

        let (image, image_tree) = child_iter.next().expect("image child");
        let image_layout = layout_iter.next().expect("image layout");
        let image_bounds = image_layout.bounds();

        if draw_hover {
            renderer.fill_quad(
                Quad {
                    bounds: image_bounds,
                    border: Border {
                        radius: self.corner_radius.into(),
                        ..Border::default()
                    },
                    shadow: style::scale_shadow(style::ELEVATION_RESTING, factor),
                    ..Quad::default()
                },
                Color::TRANSPARENT,
            );
        }

        image.as_widget().draw(
            image_tree,
            renderer,
            theme,
            style,
            image_layout,
            cursor,
            viewport,
        );

        if draw_hover {
            // Sub-layer so the tint paints over the image instead of being
            // batched underneath it.
            renderer.with_layer(image_bounds, |renderer| {
                renderer.fill_quad(
                    Quad {
                        bounds: image_bounds,
                        border: Border {
                            radius: self.corner_radius.into(),
                            ..Border::default()
                        },
                        ..Quad::default()
                    },
                    color::with_alpha(color::BG, TINT_ALPHA * factor),
                );
            });
        }

        // Pills always draw, at rest and on hover. They sit above the tint
        // so the frosted backdrop reads cleanly.
        for _ in 0..self.pill_corners.len() {
            let (child, child_tree) = child_iter.next().expect("pill child");
            let child_layout = layout_iter.next().expect("pill layout");

            renderer.with_layer(image_bounds, |renderer| {
                child.as_widget().draw(
                    child_tree,
                    renderer,
                    theme,
                    style,
                    child_layout,
                    cursor,
                    viewport,
                );
            });
        }

        if let Some(overlay_layout) = layout_iter.next() {
            let (overlay, overlay_tree) = child_iter.next().expect("overlay child");

            let draw_overlay = !self.interactive() || draw_hover;
            if draw_overlay {
                let transform = if self.interactive() {
                    let cx = image_bounds.x + image_bounds.width / 2.0;
                    let cy = image_bounds.y + image_bounds.height / 2.0;
                    Transformation::translate(cx, cy)
                        * Transformation::scale(factor)
                        * Transformation::translate(-cx, -cy)
                } else {
                    Transformation::IDENTITY
                };

                renderer.with_layer(image_bounds, |renderer| {
                    renderer.with_transformation(transform, |renderer| {
                        overlay.as_widget().draw(
                            overlay_tree,
                            renderer,
                            theme,
                            style,
                            overlay_layout,
                            cursor,
                            viewport,
                        );
                    });
                });
            }
        }

        if draw_hover && self.border {
            renderer.with_layer(image_bounds, |renderer| {
                renderer.fill_quad(
                    Quad {
                        bounds: image_bounds,
                        border: Border {
                            color: color::primary(),
                            width: style::BORDER_WIDTH * factor,
                            radius: self.corner_radius.into(),
                        },
                        ..Quad::default()
                    },
                    Color::TRANSPARENT,
                );
            });
        }
    }

    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<State>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(State::default())
    }

    fn children(&self) -> Vec<Tree> {
        self.children.iter().map(Tree::new).collect()
    }

    fn diff(&self, tree: &mut Tree) {
        tree.diff_children(&self.children);
    }

    fn operate(
        &mut self,
        tree: &mut Tree,
        layout: Layout<'_>,
        renderer: &iced::Renderer,
        operation: &mut dyn Operation,
    ) {
        let now = Instant::now();
        let overlay_alive =
            self.overlay_alive(tree, &layout, mouse::Cursor::Unavailable, now);

        let child_iter = self
            .children
            .iter_mut()
            .zip(tree.children.iter_mut())
            .zip(layout.children())
            .enumerate();

        for (idx, ((child, child_tree), child_layout)) in child_iter {
            let alive = match self.overlay_idx {
                Some(overlay) if overlay == idx => overlay_alive,
                _ => true,
            };
            if !alive {
                continue;
            }
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
        renderer: &iced::Renderer,
        clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
        viewport: &Rectangle,
    ) {
        let now = Instant::now();
        let overlay_alive = self.overlay_alive(tree, &layout, cursor, now);

        let child_iter = self
            .children
            .iter_mut()
            .zip(tree.children.iter_mut())
            .zip(layout.children())
            .enumerate();

        // Image bounds are needed for hit-testing the surface press after
        // children have had a chance to capture the event.
        let mut image_bounds = Rectangle::default();

        for (idx, ((child, child_tree), child_layout)) in child_iter {
            if idx == 0 {
                image_bounds = child_layout.bounds();
            }
            let alive = match self.overlay_idx {
                Some(overlay) if overlay == idx => overlay_alive,
                _ => true,
            };
            if !alive {
                continue;
            }
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

        if !self.interactive() {
            return;
        }

        let over = cursor.is_over(image_bounds);
        let state = tree.state.downcast_mut::<State>();
        if state.image.flip(over, now) {
            shell.request_redraw();
        }

        if shell.is_event_captured() {
            return;
        }

        match event {
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) if over => {
                state.pressed = true;
            },

            Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left))
                if state.pressed =>
            {
                state.pressed = false;

                if over && let Some(message) = self.on_press.clone() {
                    shell.publish(message);
                    shell.capture_event();
                }
            },

            Event::Window(window::Event::RedrawRequested(_))
                if state.image.animating(now) =>
            {
                shell.request_redraw();
            },

            _ => {},
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
        let now = Instant::now();
        let overlay_alive = self.overlay_alive(tree, &layout, cursor, now);

        let child_iter = self
            .children
            .iter()
            .zip(tree.children.iter())
            .zip(layout.children())
            .enumerate();

        let mut image_bounds = Rectangle::default();

        for (idx, ((child, child_tree), child_layout)) in child_iter {
            if idx == 0 {
                image_bounds = child_layout.bounds();
            }
            let alive = match self.overlay_idx {
                Some(overlay) if overlay == idx => overlay_alive,
                _ => true,
            };
            if !alive {
                continue;
            }
            let interaction = child.as_widget().mouse_interaction(
                child_tree,
                child_layout,
                cursor,
                viewport,
                renderer,
            );
            if !matches!(
                interaction,
                mouse::Interaction::None | mouse::Interaction::Idle
            ) {
                return interaction;
            }
        }

        if self.interactive() && cursor.is_over(image_bounds) {
            return mouse::Interaction::Pointer;
        }

        mouse::Interaction::None
    }
}

/// A shimmer placeholder matching the dimensions of a [`media_image`]. Use
/// while the backdrop is still loading so the page holds its shape.
pub fn media_image_skeleton<'a, Message>(size: Size, radius: f32) -> Element<'a, Message>
where
    Message: 'a,
{
    crate::widget::skeleton::skeleton()
        .width(Length::Fixed(size.width))
        .height(Length::Fixed(size.height))
        .radius(radius)
        .into()
}

/// Origin of a pill at `corner`, offset along the corner's edge by
/// `stack_offset` so multiple pills sharing a corner line up cleanly. Left
/// corners stack rightward, right corners stack leftward, so the first pill
/// added at each corner sits closest to that corner.
fn pill_origin(
    corner: PillCorner,
    image: Rectangle,
    pill: Size,
    stack_offset: f32,
) -> Point {
    match corner {
        PillCorner::TopLeft => Point::new(
            image.x + spacing::PAD_12 + stack_offset,
            image.y + spacing::PAD_12,
        ),
        PillCorner::TopRight => Point::new(
            image.x + image.width - spacing::PAD_12 - pill.width - stack_offset,
            image.y + spacing::PAD_12,
        ),
        PillCorner::BottomLeft => Point::new(
            image.x + spacing::PAD_12 + stack_offset,
            image.y + image.height - spacing::PAD_12 - pill.height,
        ),
        PillCorner::BottomRight => Point::new(
            image.x + image.width - spacing::PAD_12 - pill.width - stack_offset,
            image.y + image.height - spacing::PAD_12 - pill.height,
        ),
    }
}
