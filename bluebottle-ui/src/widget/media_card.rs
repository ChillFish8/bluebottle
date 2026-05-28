//! A media card with an image, optional label, and optional subtext.
//!
//! The image always publishes a click message. The label and subtext are
//! optional Element slots so callers can render any content. Each may set its
//! own press message. If unset they fall back to the image's. Label and
//! subtext gain a hover-animated underline when they have their own press.
//! Hovering the image animates in a primary border, a drop shadow, a
//! background tint, and scales the optional overlay element in from the
//! centre. iced 0.14 has no per-widget opacity, so the overlay uses a
//! scale-from-centre animation rather than an alpha fade.

use std::time::{Duration, Instant};

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
    border,
    mouse,
    window,
};

use crate::{color, easing, style};

/// How long every hover animation in the card takes to fade in or out.
const HOVER_FADE: Duration = Duration::from_millis(130);

/// Thickness of the hover underline, in logical pixels.
const UNDERLINE_THICKNESS: f32 = 1.0;

/// Below this factor a hover effect counts as fully hidden.
const EPSILON: f32 = 0.001;

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
/// default. Set `.on_press(...)` to make the image (and any rows without their
/// own press) clickable. Optional `.label(...)`, `.subtext(...)`, and
/// `.overlay(...)` extend the card.
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
        label_color: color::TEXT_DEFAULT,
        subtext_color: color::TEXT_SECONDARY,
        on_press: None,
        on_label_press: None,
        on_subtext_press: None,
    }
}

/// A configurable media card, built by [`media_card`].
pub struct MediaCard<'a, Message> {
    image: Element<'a, Message>,
    overlay: Option<Element<'a, Message>>,
    label: Option<Element<'a, Message>>,
    subtext: Option<Element<'a, Message>>,
    label_color: Color,
    subtext_color: Color,
    on_press: Option<Message>,
    on_label_press: Option<Message>,
    on_subtext_press: Option<Message>,
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

    /// Sets the colour of the label's hover underline. Defaults to
    /// [`color::TEXT_DEFAULT`] so it matches the standard label typography.
    pub fn label_color(mut self, color: Color) -> Self {
        self.label_color = color;
        self
    }

    /// Sets the colour of the subtext's hover underline. Defaults to
    /// [`color::TEXT_SECONDARY`] so it matches the standard subtext typography.
    pub fn subtext_color(mut self, color: Color) -> Self {
        self.subtext_color = color;
        self
    }

    /// Sets a press message for the image. Also the fallback message for any
    /// label or subtext without its own press.
    pub fn on_press(mut self, message: Message) -> Self {
        self.on_press = Some(message);
        self
    }

    /// Sets a press message for the label. The label gains a hover underline.
    pub fn on_label_press(mut self, message: Message) -> Self {
        self.on_label_press = Some(message);
        self
    }

    /// Sets a press message for the subtext. The subtext gains a hover
    /// underline.
    pub fn on_subtext_press(mut self, message: Message) -> Self {
        self.on_subtext_press = Some(message);
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
            label_color: card.label_color,
            subtext_color: card.subtext_color,
            on_press: card.on_press,
            on_label_press: card.on_label_press,
            on_subtext_press: card.on_subtext_press,
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

/// The actual widget. Holds the (already-composed) image element and the two
/// optional row elements. Lays them out manually so it can publish per-region
/// presses and draw hover underlines.
struct Card<'a, Message> {
    children: Vec<Element<'a, Message>>,
    slots: Vec<Slot>,
    label_color: Color,
    subtext_color: Color,
    on_press: Option<Message>,
    on_label_press: Option<Message>,
    on_subtext_press: Option<Message>,
}

impl<'a, Message: Clone> Card<'a, Message> {
    /// Message to publish when the given slot is clicked, or `None` if the
    /// slot has no message and no fallback. The overlay never publishes from
    /// the card itself, its interactive children publish their own messages.
    fn press_for(&self, slot: Slot) -> Option<Message> {
        match slot {
            Slot::Image => self.on_press.clone(),
            Slot::Overlay => None,
            Slot::Label => self
                .on_label_press
                .clone()
                .or_else(|| self.on_press.clone()),
            Slot::Subtext => self
                .on_subtext_press
                .clone()
                .or_else(|| self.on_press.clone()),
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
    fn layout_of<'l>(
        &self,
        slot: Slot,
        layout: &Layout<'l>,
    ) -> Option<Layout<'l>> {
        self.slots
            .iter()
            .zip(layout.children())
            .find(|(s, _)| **s == slot)
            .map(|(_, l)| l)
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Slot {
    Image,
    Overlay,
    Label,
    Subtext,
}

/// The eased hover animation for one region. `current(now)` reads the live
/// factor and `flip(hovering, now)` retargets without snapping. Storing
/// `from`/`target`/`started` lets a mid-flight reversal continue from where
/// the previous one left off. See sidebar's `State` for the same pattern.
#[derive(Clone, Copy)]
struct HoverAnim {
    from: f32,
    target: f32,
    started: Instant,
}

impl Default for HoverAnim {
    fn default() -> Self {
        Self {
            from: 0.0,
            target: 0.0,
            started: Instant::now() - HOVER_FADE,
        }
    }
}

impl HoverAnim {
    /// The factor right now, eased from `from` toward `target` over
    /// `HOVER_FADE`. Fade-in uses an emphasized decelerate (fast start, soft
    /// settle) so the border, tint, shadow, and overlay reveal land within
    /// the first half of the animation instead of bunching at the end.
    /// Fade-out uses the matching accelerate curve, mirroring sidebar.
    fn current(&self, now: Instant) -> f32 {
        let raw = (now.duration_since(self.started).as_secs_f32()
            / HOVER_FADE.as_secs_f32())
        .clamp(0.0, 1.0);
        let curve = if self.target >= self.from {
            &easing::EMPHASIZED_DECELERATE
        } else {
            &easing::EMPHASIZED_ACCELERATE
        };
        let eased = curve.y_at_x(raw);
        self.from + (self.target - self.from) * eased
    }

    /// Retargets to 1.0 if hovering, else 0.0. The new animation starts from
    /// the live factor, so a reversal mid-flight is smooth.
    fn flip(&mut self, hovering: bool, now: Instant) {
        let target = if hovering { 1.0 } else { 0.0 };
        if target == self.target {
            return;
        }
        self.from = self.current(now);
        self.target = target;
        self.started = now;
    }

    /// Whether the region still has movement left.
    fn animating(&self, now: Instant) -> bool {
        now.duration_since(self.started) < HOVER_FADE
    }
}

#[derive(Clone, Copy, Default)]
struct State {
    image: HoverAnim,
    image_hovering: bool,
    label: HoverAnim,
    label_hovering: bool,
    subtext: HoverAnim,
    subtext_hovering: bool,
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
        let mut nodes: Vec<Option<layout::Node>> = (0..self.children.len()).map(|_| None).collect();
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
                    positioned
                        .push(node.move_to(Point::new(CARD_PADDING, image_y)));
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

        let total =
            Size::new(max_row_width + CARD_PADDING * 2.0, y + CARD_PADDING);

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
                    shadow: style::scale_shadow(
                        style::ELEVATION_SHADOW,
                        image_factor,
                    ),
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
                        color::with_alpha(
                            color::BACKGROUND,
                            TINT_ALPHA * image_factor,
                        ),
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

        // Underlines for the text rows.
        for (slot, child_layout) in self.slots.iter().zip(layout.children()) {
            let (anim, tint) = match slot {
                Slot::Label if self.on_label_press.is_some() => {
                    (&state.label, self.label_color)
                },
                Slot::Subtext if self.on_subtext_press.is_some() => {
                    (&state.subtext, self.subtext_color)
                },
                _ => continue,
            };

            let factor = anim.current(now);
            if factor <= EPSILON {
                continue;
            }

            let bounds = child_layout.bounds();
            let line = Rectangle {
                x: bounds.x,
                y: bounds.y + bounds.height,
                width: bounds.width * factor,
                height: UNDERLINE_THICKNESS,
            };

            renderer.fill_quad(
                Quad {
                    bounds: line,
                    border: border::rounded(UNDERLINE_THICKNESS / 2.0),
                    ..Quad::default()
                },
                color::with_alpha(tint, tint.a * factor),
            );
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
        let overlay_alive =
            tree.state.downcast_ref::<State>().image.current(Instant::now()) > EPSILON;

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
        let image_bounds = self.layout_of(Slot::Image, &layout).map(|l| l.bounds());
        let cursor_over_image = image_bounds
            .is_some_and(|b| cursor.is_over(b));
        // Forward to overlay whenever it is at least partly visible OR the
        // cursor is currently over the image bounds. The second condition
        // catches the very first CursorMoved on hover-in, before the animation
        // has had a chance to lift the factor above EPSILON. Without it, the
        // overlay's interactive children would miss the entry event and start
        // a frame late on their own hover state.
        let overlay_alive = {
            let state = tree.state.downcast_ref::<State>();
            state.image.current(now) > EPSILON
                || (self.image_interactive() && cursor_over_image)
        };

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

        let state = tree.state.downcast_mut::<State>();

        if let Event::Mouse(mouse::Event::CursorMoved { .. }) = event {
            // Image hover drives the border, shadow, tint, and overlay.
            if self.image_interactive()
                && let Some(image_layout) = self.layout_of(Slot::Image, &layout)
            {
                let over = cursor.is_over(image_layout.bounds());
                if over != state.image_hovering {
                    state.image_hovering = over;
                    state.image.flip(over, now);
                    shell.request_redraw();
                }
            }

            for (slot, child_layout) in self.slots.iter().zip(layout.children()) {
                let over = cursor.is_over(child_layout.bounds());

                match slot {
                    Slot::Label
                        if self.on_label_press.is_some()
                            && over != state.label_hovering =>
                    {
                        state.label_hovering = over;
                        state.label.flip(over, now);
                        shell.request_redraw();
                    },
                    Slot::Subtext
                        if self.on_subtext_press.is_some()
                            && over != state.subtext_hovering =>
                    {
                        state.subtext_hovering = over;
                        state.subtext.flip(over, now);
                        shell.request_redraw();
                    },
                    _ => {},
                }
            }
        }

        if shell.is_event_captured() {
            return;
        }

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

            Event::Window(window::Event::RedrawRequested(_)) => {
                let image_animating =
                    self.image_interactive() && state.image.animating(now);
                let label_animating =
                    self.on_label_press.is_some() && state.label.animating(now);
                let subtext_animating =
                    self.on_subtext_press.is_some() && state.subtext.animating(now);

                if image_animating || label_animating || subtext_animating {
                    shell.request_redraw();
                }
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
        let image_bounds = self.layout_of(Slot::Image, &layout).map(|l| l.bounds());
        let cursor_over_image = image_bounds
            .is_some_and(|b| cursor.is_over(b));
        let overlay_alive = tree
            .state
            .downcast_ref::<State>()
            .image
            .current(Instant::now())
            > EPSILON
            || (self.image_interactive() && cursor_over_image);

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
