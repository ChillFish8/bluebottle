//! A media card with an image, optional label, and optional subtext.
//!
//! The image publishes a click message. The label and subtext are optional
//! Element slots so callers can render any content. A click anywhere on the
//! card that is not captured by an interactive child (a [`link`] in the label
//! slot, an overlay button) publishes the image's press message. For a
//! clickable label or subtext with its own message and hover-underline, pass
//! a [`link`](super::link::link) element. Hovering the image animates in a
//! primary border, a drop shadow, a background tint, and scales the optional
//! overlay element in from the centre. iced 0.14 has no per-widget opacity,
//! so the overlay uses a scale-from-centre animation rather than an alpha
//! fade.

use std::time::Instant;

use iced::advanced::renderer::{Quad, Style};
use iced::advanced::widget::{Operation, Tree, tree};
use iced::advanced::{Clipboard, Layout, Renderer, Shell, Widget, layout};
use iced::widget::{column, container, row, space};
use iced::{
    Border,
    Center,
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
use crate::{color, style};

/// Vertical gap between the image, label, and subtext.
const ROW_SPACING: f32 = 4.0;

/// Outer padding around the whole card. Leaves room for the focus border and
/// the drop shadow that paint just outside the image's bounds.
const CARD_PADDING: f32 = 2.0;

/// Border width around the image at full hover, in logical pixels.
const BORDER_WIDTH: f32 = 1.0;

/// Background-tint alpha applied over the image at full hover.
const TINT_ALPHA: f32 = 0.75;

/// Corner radius the hover effects use. Matches the default image rounding so
/// the border, tint, and shadow trace the same shape.
const HOVER_RADIUS: f32 = super::skeleton::DEFAULT_RADIUS;

/// Creates a media card around `image`. The card is non-interactive by
/// default. Set `.on_press(...)` to make the card publish a message when any
/// part of it is clicked. Optional `.label(...)`, `.subtext(...)`, and
/// `.overlay(...)` extend the card. For a clickable label or subtext with its
/// own message and hover-animated underline, pass a
/// [`link`](super::link::link) element in that slot.
pub fn media_card<'a, Message>(
    image: impl Into<Element<'a, Message>>,
) -> MediaCard<'a, Message>
where
    Message: Clone + 'a,
{
    MediaCard {
        image: image.into(),
        overlay: None,
        label: None,
        subtext: None,
        on_press: None,
    }
}

/// A configurable media card, built by [`media_card`].
pub struct MediaCard<'a, Message> {
    image: Element<'a, Message>,
    overlay: Option<Element<'a, Message>>,
    label: Option<Element<'a, Message>>,
    subtext: Option<Element<'a, Message>>,
    on_press: Option<Message>,
}

impl<'a, Message> MediaCard<'a, Message>
where
    Message: Clone + 'a,
{
    /// Adds a label row below the image.
    pub fn label(mut self, label: impl Into<Element<'a, Message>>) -> Self {
        self.label = Some(label.into());
        self
    }

    /// Adds a subtext row below the label.
    pub fn subtext(mut self, subtext: impl Into<Element<'a, Message>>) -> Self {
        self.subtext = Some(subtext.into());
        self
    }

    /// Layers `overlay` on top of the image, revealed by iced's hover wrapper.
    pub fn overlay(mut self, overlay: impl Into<Element<'a, Message>>) -> Self {
        self.overlay = Some(overlay.into());
        self
    }

    /// Sets the press message for the card. Any click that is not captured by
    /// an interactive child (a [`link`](super::link::link) in the label slot,
    /// an overlay button) publishes this message.
    pub fn on_press(mut self, message: Message) -> Self {
        self.on_press = Some(message);
        self
    }
}

impl<'a, Message> From<MediaCard<'a, Message>> for Element<'a, Message>
where
    Message: Clone + 'a,
{
    fn from(card: MediaCard<'a, Message>) -> Self {
        let mut children: Vec<Element<'a, Message>> = vec![card.image];
        let mut slots: Vec<Slot> = vec![Slot::Image];

        if let Some(overlay) = card.overlay {
            children.push(overlay);
            slots.push(Slot::Overlay);
        }
        if let Some(label) = card.label {
            children.push(label);
            slots.push(Slot::Label);
        }
        if let Some(subtext) = card.subtext {
            children.push(subtext);
            slots.push(Slot::Subtext);
        }

        Element::new(Card {
            children,
            slots,
            on_press: card.on_press,
        })
    }
}

/// Creates a skeleton placeholder for a media card. The display element is
/// rendered as-is. `.label()` and `.subtext()` toggle stand-in shimmer rows so
/// the placeholder lines up with whichever rows the real card will show.
pub fn skeleton<'a, Message>(
    display: impl Into<Element<'a, Message>>,
) -> Skeleton<'a, Message>
where
    Message: Clone + 'a,
{
    Skeleton {
        display: display.into(),
        label: false,
        subtext: false,
    }
}

/// A configurable skeleton, built by [`skeleton`].
pub struct Skeleton<'a, Message> {
    display: Element<'a, Message>,
    label: bool,
    subtext: bool,
}

impl<'a, Message> Skeleton<'a, Message>
where
    Message: Clone + 'a,
{
    /// Shows a shimmer placeholder where the label row would sit.
    pub fn label(mut self) -> Self {
        self.label = true;
        self
    }

    /// Shows a shimmer placeholder where the subtext row would sit.
    pub fn subtext(mut self) -> Self {
        self.subtext = true;
        self
    }
}

impl<'a, Message> From<Skeleton<'a, Message>> for Element<'a, Message>
where
    Message: Clone + 'a,
{
    fn from(s: Skeleton<'a, Message>) -> Self {
        let label = row![
            super::skeleton::skeleton()
                .height(14)
                .radius(4.0)
                .width(Length::FillPortion(4)),
            space().width(Length::FillPortion(2)),
        ]
        .align_y(Center);

        let subtext = row![
            super::skeleton::skeleton()
                .height(12)
                .radius(4.0)
                .width(Length::FillPortion(2)),
            space().width(Length::FillPortion(2)),
        ]
        .align_y(Center);

        let mut base = column![s.display].spacing(ROW_SPACING);
        if s.label {
            base = base.push(label);
        }
        if s.subtext {
            base = base.push(subtext);
        }

        let wrapper = container(base).width(Length::Shrink);
        container(wrapper).padding(CARD_PADDING).into()
    }
}

/// The actual widget. Holds the image, an optional overlay, and the two
/// optional row elements. Lays them out manually so it can publish a single
/// press message for any non-captured click and animate image hover effects.
struct Card<'a, Message> {
    children: Vec<Element<'a, Message>>,
    slots: Vec<Slot>,
    on_press: Option<Message>,
}

impl<'a, Message: Clone> Card<'a, Message> {
    /// Message to publish when the given slot is clicked, or `None` if the
    /// slot has no message. The overlay never publishes from the card itself,
    /// its interactive children publish their own messages.
    fn press_for(&self, slot: Slot) -> Option<Message> {
        match slot {
            Slot::Image | Slot::Label | Slot::Subtext => self.on_press.clone(),
            Slot::Overlay => None,
        }
    }

    /// Whether the card's image should react to hover. True when anything
    /// about the image is interactive, the image itself or an overlay sitting
    /// on top.
    fn image_interactive(&self) -> bool {
        self.on_press.is_some() || self.has_slot(Slot::Overlay)
    }

    fn has_slot(&self, slot: Slot) -> bool {
        self.slots.contains(&slot)
    }

    /// The layout node for `slot`, if any.
    fn layout_of<'l>(&self, slot: Slot, layout: &Layout<'l>) -> Option<Layout<'l>> {
        self.slots
            .iter()
            .zip(layout.children())
            .find(|(s, _)| **s == slot)
            .map(|(_, l)| l)
    }

    /// Whether the overlay should receive events, focus, and pointer
    /// feedback. True while the image hover factor is at least slightly
    /// visible, or (on the entering frame, before the factor has lifted past
    /// `EPSILON`) while the cursor is currently over an interactive image.
    /// Centralised so update/operate/mouse_interaction share one rule.
    fn overlay_alive(
        &self,
        tree: &Tree,
        layout: &Layout<'_>,
        cursor: mouse::Cursor,
        now: Instant,
    ) -> bool {
        let cursor_over_image = self
            .layout_of(Slot::Image, layout)
            .is_some_and(|l| cursor.is_over(l.bounds()));
        let factor = tree.state.downcast_ref::<State>().image.current(now);
        factor > EPSILON || (self.image_interactive() && cursor_over_image)
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Slot {
    Image,
    Overlay,
    Label,
    Subtext,
}

#[derive(Clone, Copy, Default)]
struct State {
    image: Hover,
    /// Whether a left button press started over the card. Releases without a
    /// matching press are ignored.
    pressed: bool,
}

impl<'a, Message> Widget<Message, iced::Theme, iced::Renderer> for Card<'a, Message>
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
        let inner = limits.shrink(Size::new(CARD_PADDING * 2.0, CARD_PADDING * 2.0));

        // Image first so the overlay knows its target size.
        let mut nodes: Vec<Option<layout::Node>> =
            (0..self.children.len()).map(|_| None).collect();
        let mut image_size = Size::ZERO;

        for (i, (child, child_tree)) in self
            .children
            .iter_mut()
            .zip(tree.children.iter_mut())
            .enumerate()
        {
            if self.slots[i] == Slot::Image {
                let node = child.as_widget_mut().layout(child_tree, renderer, &inner);
                image_size = node.size();
                nodes[i] = Some(node);
            }
        }

        for (i, (child, child_tree)) in self
            .children
            .iter_mut()
            .zip(tree.children.iter_mut())
            .enumerate()
        {
            if nodes[i].is_some() {
                continue;
            }
            let node = match self.slots[i] {
                Slot::Overlay => {
                    // Constrain the overlay to the image's exact bounds.
                    let limits = layout::Limits::new(image_size, image_size);
                    child.as_widget_mut().layout(child_tree, renderer, &limits)
                },
                _ => child.as_widget_mut().layout(child_tree, renderer, &inner),
            };
            nodes[i] = Some(node);
        }

        let nodes: Vec<layout::Node> =
            nodes.into_iter().map(|n| n.expect("laid out")).collect();

        // Row width excludes the overlay since it tracks the image.
        let max_row_width = self
            .slots
            .iter()
            .zip(nodes.iter())
            .filter(|(s, _)| **s != Slot::Overlay)
            .map(|(_, n)| n.size().width)
            .fold(0.0_f32, f32::max);

        let mut y = CARD_PADDING;
        let mut image_y = y;
        let mut started_rows = false;
        let mut positioned: Vec<layout::Node> = Vec::with_capacity(nodes.len());

        for (slot, node) in self.slots.iter().zip(nodes) {
            match slot {
                Slot::Image => {
                    image_y = y;
                    let h = node.size().height;
                    positioned.push(node.move_to(Point::new(CARD_PADDING, y)));
                    y += h;
                    started_rows = true;
                },
                Slot::Overlay => {
                    // Overlay sits on the image, not in the row stack.
                    positioned.push(node.move_to(Point::new(CARD_PADDING, image_y)));
                },
                Slot::Label | Slot::Subtext => {
                    if started_rows {
                        y += ROW_SPACING;
                    }
                    let h = node.size().height;
                    positioned.push(node.move_to(Point::new(CARD_PADDING, y)));
                    y += h;
                },
            }
        }

        let total = Size::new(max_row_width + CARD_PADDING * 2.0, y + CARD_PADDING);

        layout::Node::with_children(total, positioned)
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
        let now = Instant::now();
        let image_factor = state.image.current(now);
        let image_bounds = self.layout_of(Slot::Image, &layout).map(|l| l.bounds());

        // Shadow behind the image. The fill is transparent, so only the
        // shadow shows; the image itself paints over it next.
        if image_factor > EPSILON
            && let Some(bounds) = image_bounds
        {
            renderer.fill_quad(
                Quad {
                    bounds,
                    border: Border {
                        radius: HOVER_RADIUS.into(),
                        ..Border::default()
                    },
                    shadow: style::scale_shadow(style::ELEVATION_SHADOW, image_factor),
                    ..Quad::default()
                },
                Color::TRANSPARENT,
            );
        }

        // Children. Image, optional overlay (only when hovered), label,
        // subtext.
        for ((slot, child), (child_tree, child_layout)) in self
            .slots
            .iter()
            .zip(self.children.iter())
            .zip(tree.children.iter().zip(layout.children()))
        {
            if *slot == Slot::Overlay && image_factor <= EPSILON {
                continue;
            }

            // iced 0.14 has no per-widget opacity primitive, so the
            // overlay's reveal is animated by scaling its drawn output from
            // the centre of the image, clipped to the image bounds. At
            // factor 0 the scale is zero (collapsed to a point) and at
            // factor 1 the overlay is at its natural size. A pure mask-based
            // opacity fade washed the whole image to the background colour
            // on the first frame of hover, which read as a snap. The scale
            // animates from invisible to full smoothly across every frame.
            if *slot == Slot::Overlay
                && let Some(image_b) = image_bounds
            {
                let cx = image_b.x + image_b.width / 2.0;
                let cy = image_b.y + image_b.height / 2.0;
                let transform = Transformation::translate(cx, cy)
                    * Transformation::scale(image_factor)
                    * Transformation::translate(-cx, -cy);

                renderer.with_layer(image_b, |renderer| {
                    renderer.with_transformation(transform, |renderer| {
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
                });
                continue;
            }

            child.as_widget().draw(
                child_tree,
                renderer,
                theme,
                style,
                child_layout,
                cursor,
                viewport,
            );

            // Tint over the image. iced batches every quad before any image
            // in the same layer, so we push a sub-layer to defer the tint
            // past the image's draw. This sub-layer is registered before the
            // overlay's, so the overlay sits above the tint.
            if *slot == Slot::Image
                && image_factor > EPSILON
                && let Some(b) = image_bounds
            {
                renderer.with_layer(b, |renderer| {
                    renderer.fill_quad(
                        Quad {
                            bounds: b,
                            border: Border {
                                radius: HOVER_RADIUS.into(),
                                ..Border::default()
                            },
                            ..Quad::default()
                        },
                        color::with_alpha(color::BACKGROUND, TINT_ALPHA * image_factor),
                    );
                });
            }
        }

        // Animated border, painted inside the image's bounds so it overlays
        // the outermost pixels of the artwork. Pushed as its own sub-layer
        // after the overlay's sub-layer, so the border sits on top of both
        // the image and the overlay.
        if image_factor > EPSILON
            && let Some(bounds) = image_bounds
        {
            renderer.with_layer(bounds, |renderer| {
                renderer.fill_quad(
                    Quad {
                        bounds,
                        border: Border {
                            color: color::PRIMARY,
                            width: BORDER_WIDTH * image_factor,
                            radius: HOVER_RADIUS.into(),
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
        // Skip the overlay when it is fully hidden so focus traversal cannot
        // land on an invisible interactive widget. iced runs `operate` for
        // things like Tab focus and accessibility scans, both of which would
        // otherwise visit children the user cannot see.
        let overlay_alive = tree
            .state
            .downcast_ref::<State>()
            .image
            .current(Instant::now())
            > EPSILON;

        for ((slot, child), (child_tree, child_layout)) in self
            .slots
            .iter()
            .zip(self.children.iter_mut())
            .zip(tree.children.iter_mut().zip(layout.children()))
        {
            if *slot == Slot::Overlay && !overlay_alive {
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

        // Forward to children first so anything interactive inside (e.g. an
        // overlay button) can capture the event. The overlay only sees events
        // while it is visible, so its inner widgets do not respond to clicks
        // they cannot be seen for.
        for ((slot, child), (child_tree, child_layout)) in self
            .slots
            .iter()
            .zip(self.children.iter_mut())
            .zip(tree.children.iter_mut().zip(layout.children()))
        {
            if *slot == Slot::Overlay && !overlay_alive {
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

        // Reconcile image hover with the live cursor on every event, not just
        // CursorMoved. This catches the case where a scroll or layout change
        // moves the card out from under (or back under) a stationary cursor
        // without iced emitting a CursorMoved. `Hover::flip` is idempotent
        // and reports back when the target actually changes.
        if self.image_interactive()
            && let Some(image_layout) = self.layout_of(Slot::Image, &layout)
        {
            let over = cursor.is_over(image_layout.bounds());
            let state = tree.state.downcast_mut::<State>();
            if state.image.flip(over, now) {
                shell.request_redraw();
            }
        }

        if shell.is_event_captured() {
            return;
        }

        let state = tree.state.downcast_mut::<State>();

        match event {
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left))
                if cursor.is_over(layout.bounds()) =>
            {
                state.pressed = true;
            },

            Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)) => {
                if !state.pressed {
                    return;
                }
                state.pressed = false;

                for (slot, child_layout) in self.slots.iter().zip(layout.children()) {
                    if *slot == Slot::Overlay {
                        continue;
                    }
                    if !cursor.is_over(child_layout.bounds()) {
                        continue;
                    }

                    if let Some(message) = self.press_for(*slot) {
                        shell.publish(message);
                        shell.capture_event();
                    }
                    break;
                }
            },

            Event::Window(window::Event::RedrawRequested(_))
                if self.image_interactive() && state.image.animating(now) =>
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

        // If a visible child wants a non-default cursor, let it win.
        for ((slot, child), (child_tree, child_layout)) in self
            .slots
            .iter()
            .zip(self.children.iter())
            .zip(tree.children.iter().zip(layout.children()))
        {
            if *slot == Slot::Overlay && !overlay_alive {
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

        // Otherwise any region that produces a press shows the pointer hand.
        for (slot, child_layout) in self.slots.iter().zip(layout.children()) {
            if cursor.is_over(child_layout.bounds()) && self.press_for(*slot).is_some() {
                return mouse::Interaction::Pointer;
            }
        }

        mouse::Interaction::None
    }
}
