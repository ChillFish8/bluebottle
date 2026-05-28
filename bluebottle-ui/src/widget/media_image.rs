//! A wrapped image with optional overlay and optional press message.
//!
//! With no `on_press` set the image is fully inert. No hover affordances
//! animate, the pointer cursor stays default, and clicks are not captured.
//! Any overlay supplied is drawn at rest, fully visible, so callers can use
//! the widget as a static composition primitive.
//!
//! With `on_press` set, hovering animates a drop shadow under the image, a
//! background tint over it, and scales the optional overlay in from the
//! centre. The primary border around the image is animated as well by
//! default and can be turned off independently with `.border(false)`. The
//! shadow, tint, and overlay scale-in always animate when `on_press` is
//! set, regardless of the border setting. iced 0.14 has no per-widget
//! opacity, so the overlay uses a scale-from-centre animation rather than
//! an alpha fade.

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

// Reuses the same rounding the image module applies, so the hover border,
// tint, and shadow trace exactly the same shape as the painted image.
use super::skeleton::DEFAULT_RADIUS as IMAGE_RADIUS;
use crate::animate::hover::{EPSILON, Hover};
use crate::{color, style};

/// Border width around the image at full hover, in logical pixels.
const BORDER_WIDTH: f32 = 1.0;

/// Background-tint alpha applied over the image at full hover.
const TINT_ALPHA: f32 = 0.75;

/// Creates a media image around `image`. Non-interactive by default. Set
/// `.on_press(...)` to make a click publish a message and enable the hover
/// affordances. Add `.overlay(...)` to layer content on top of the image.
pub fn media_image<'a, Message>(
    image: impl Into<Element<'a, Message>>,
) -> MediaImage<'a, Message>
where
    Message: Clone + 'a,
{
    MediaImage {
        image: image.into(),
        overlay: None,
        on_press: None,
        border: true,
    }
}

/// A configurable media image, built by [`media_image`].
pub struct MediaImage<'a, Message> {
    image: Element<'a, Message>,
    overlay: Option<Element<'a, Message>>,
    on_press: Option<Message>,
    border: bool,
}

impl<'a, Message> MediaImage<'a, Message>
where
    Message: Clone + 'a,
{
    /// Layers `overlay` on top of the image. When `on_press` is set the
    /// overlay scales in from the centre as the cursor enters and out as it
    /// leaves. Without `on_press` the overlay is drawn at rest.
    pub fn overlay(mut self, overlay: impl Into<Element<'a, Message>>) -> Self {
        self.overlay = Some(overlay.into());
        self
    }

    /// Sets the press message. Required to enable the hover affordances and
    /// the pointer cursor. Without one the widget is inert.
    pub fn on_press(mut self, message: Message) -> Self {
        self.on_press = Some(message);
        self
    }

    /// Toggles the primary hover border around the image. Defaults to true.
    /// Has no effect when `on_press` is unset, since the inert image has no
    /// hover affordances at all.
    pub fn border(mut self, enabled: bool) -> Self {
        self.border = enabled;
        self
    }
}

impl<'a, Message> From<MediaImage<'a, Message>> for Element<'a, Message>
where
    Message: Clone + 'a,
{
    fn from(card: MediaImage<'a, Message>) -> Self {
        let mut children: Vec<Element<'a, Message>> = vec![card.image];
        if let Some(overlay) = card.overlay {
            children.push(overlay);
        }
        Element::new(Inner {
            children,
            on_press: card.on_press,
            border: card.border,
        })
    }
}

/// The image is always at index 0. The optional overlay is at index 1 when
/// present. Stored as a Vec so the trait-level `diff_children` can take it as
/// a contiguous slice; the rest of the widget accesses the two slots through
/// named destructuring rather than indexed iteration.
struct Inner<'a, Message> {
    children: Vec<Element<'a, Message>>,
    on_press: Option<Message>,
    border: bool,
}

impl<'a, Message: Clone> Inner<'a, Message> {
    fn interactive(&self) -> bool {
        self.on_press.is_some()
    }

    fn has_overlay(&self) -> bool {
        self.children.len() > 1
    }

    fn image_bounds(&self, layout: &Layout<'_>) -> Rectangle {
        layout.children().next().expect("image child").bounds()
    }

    /// Whether the overlay should receive events, focus, and pointer
    /// feedback. True while the hover factor is above `EPSILON`, or (on the
    /// entering frame, before the factor has lifted) while the cursor is
    /// over an interactive image.
    fn overlay_alive(
        &self,
        tree: &Tree,
        layout: &Layout<'_>,
        cursor: mouse::Cursor,
        now: Instant,
    ) -> bool {
        if !self.has_overlay() {
            return false;
        }
        // Non-interactive overlays are drawn at rest, so they are always
        // alive for events and operate.
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
    /// Whether a left button press started over the image. Releases without
    /// a matching press are ignored.
    pressed: bool,
}

impl<'a, Message> Widget<Message, iced::Theme, iced::Renderer> for Inner<'a, Message>
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
        let (image, rest) = self.children.split_first_mut().expect("image child");
        let (image_tree, rest_trees) =
            tree.children.split_first_mut().expect("image tree");

        let image_node = image
            .as_widget_mut()
            .layout(image_tree, renderer, limits)
            .move_to(Point::ORIGIN);
        let image_size = image_node.size();

        let overlay_node = rest.first_mut().zip(rest_trees.first_mut()).map(
            |(overlay, overlay_tree)| {
                let overlay_limits = layout::Limits::new(image_size, image_size);
                overlay
                    .as_widget_mut()
                    .layout(overlay_tree, renderer, &overlay_limits)
                    .move_to(Point::ORIGIN)
            },
        );

        let positioned = match overlay_node {
            None => vec![image_node],
            Some(o) => vec![image_node, o],
        };
        layout::Node::with_children(image_size, positioned)
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
        // Only animate hover affordances when interactive. The inert image
        // draws with factor pinned to zero so no shadow, tint, or border
        // paint over it.
        let factor = if self.interactive() {
            state.image.current(Instant::now())
        } else {
            0.0
        };

        let mut child_iter = self.children.iter().zip(tree.children.iter());
        let mut layout_iter = layout.children();
        let (image, image_tree) = child_iter.next().expect("image child");
        let image_layout = layout_iter.next().expect("image layout");
        let image_bounds = image_layout.bounds();
        let overlay = child_iter.next();
        let overlay_layout = layout_iter.next();

        // Hover layers when interactive. One guard, four affordances. The
        // image paints between shadow (below) and the rest (above).
        let draw_hover_layers = factor > EPSILON;

        if draw_hover_layers {
            // Shadow behind the image. The fill is transparent so only the
            // shadow shows; the image itself paints over it next.
            renderer.fill_quad(
                Quad {
                    bounds: image_bounds,
                    border: Border {
                        radius: IMAGE_RADIUS.into(),
                        ..Border::default()
                    },
                    shadow: style::scale_shadow(style::ELEVATION_SHADOW, factor),
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

        if draw_hover_layers {
            // Tint over the image. iced batches every quad before any image
            // in the same layer, so we push a sub-layer to defer the tint
            // past the image's draw. The overlay's sub-layer is registered
            // after this so the overlay sits above the tint.
            renderer.with_layer(image_bounds, |renderer| {
                renderer.fill_quad(
                    Quad {
                        bounds: image_bounds,
                        border: Border {
                            radius: IMAGE_RADIUS.into(),
                            ..Border::default()
                        },
                        ..Quad::default()
                    },
                    color::with_alpha(color::BACKGROUND, TINT_ALPHA * factor),
                );
            });
        }

        // Overlay. Interactive overlays scale in from the image centre as
        // the hover factor lifts; inert overlays draw at rest at full size.
        // Both paths clip to the image bounds so overlay contents that
        // extend past the artwork are cropped consistently.
        if let Some((overlay, overlay_tree)) = overlay
            && let Some(overlay_layout) = overlay_layout
        {
            let draw_overlay = !self.interactive() || draw_hover_layers;
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

        if draw_hover_layers && self.border {
            // Animated border, painted inside the image's bounds so it
            // overlays the outermost pixels of the artwork. Pushed as its
            // own sub-layer after the overlay's sub-layer so the border
            // sits on top of both the image and the overlay.
            renderer.with_layer(image_bounds, |renderer| {
                renderer.fill_quad(
                    Quad {
                        bounds: image_bounds,
                        border: Border {
                            color: color::PRIMARY,
                            width: BORDER_WIDTH * factor,
                            radius: IMAGE_RADIUS.into(),
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
        let overlay_alive = if self.interactive() {
            tree.state
                .downcast_ref::<State>()
                .image
                .current(Instant::now())
                > EPSILON
        } else {
            true
        };

        let mut child_iter = self
            .children
            .iter_mut()
            .zip(tree.children.iter_mut())
            .zip(layout.children());
        let ((image, image_tree), image_layout) =
            child_iter.next().expect("image child");
        image
            .as_widget_mut()
            .operate(image_tree, image_layout, renderer, operation);

        if let Some(((overlay, overlay_tree), overlay_layout)) = child_iter.next()
            && overlay_alive
        {
            overlay.as_widget_mut().operate(
                overlay_tree,
                overlay_layout,
                renderer,
                operation,
            );
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

        // Forward to children first so anything interactive inside (e.g. an
        // overlay button) can capture the event. The overlay only sees
        // events while it is visible so its inner widgets do not respond
        // to clicks they cannot be seen for.
        let mut child_iter = self
            .children
            .iter_mut()
            .zip(tree.children.iter_mut())
            .zip(layout.children());
        let ((image, image_tree), image_layout) =
            child_iter.next().expect("image child");
        let image_bounds = image_layout.bounds();

        image.as_widget_mut().update(
            image_tree,
            event,
            image_layout,
            cursor,
            renderer,
            clipboard,
            shell,
            viewport,
        );

        if let Some(((overlay, overlay_tree), overlay_layout)) = child_iter.next()
            && overlay_alive
        {
            overlay.as_widget_mut().update(
                overlay_tree,
                event,
                overlay_layout,
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

        // Reconcile image hover with the live cursor on every event, not
        // just CursorMoved. This catches the case where a scroll or layout
        // change moves the image out from under (or back under) a stationary
        // cursor without iced emitting a CursorMoved. `Hover::flip` is
        // idempotent and reports back when the target actually changes.
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
        let overlay_alive = self.overlay_alive(tree, &layout, cursor, Instant::now());

        let mut child_iter = self
            .children
            .iter()
            .zip(tree.children.iter())
            .zip(layout.children());
        let ((image, image_tree), image_layout) =
            child_iter.next().expect("image child");
        let image_bounds = image_layout.bounds();

        let image_interaction = image.as_widget().mouse_interaction(
            image_tree,
            image_layout,
            cursor,
            viewport,
            renderer,
        );
        if !matches!(
            image_interaction,
            mouse::Interaction::None | mouse::Interaction::Idle
        ) {
            return image_interaction;
        }

        if let Some(((overlay, overlay_tree), overlay_layout)) = child_iter.next()
            && overlay_alive
        {
            let overlay_interaction = overlay.as_widget().mouse_interaction(
                overlay_tree,
                overlay_layout,
                cursor,
                viewport,
                renderer,
            );
            if !matches!(
                overlay_interaction,
                mouse::Interaction::None | mouse::Interaction::Idle
            ) {
                return overlay_interaction;
            }
        }

        if self.interactive() && cursor.is_over(image_bounds) {
            return mouse::Interaction::Pointer;
        }

        mouse::Interaction::None
    }
}
