//! Shared chassis behind the new media cards (`EpisodeStill`, `PosterCard`,
//! `AlbumCard`). The frame composes the `blurred_image` surface with a
//! `ChromeOverlay` widget on top via `iced::widget::stack`, so the chrome
//! (play, heart, watched checkbox, time-left pill, progress strip, accent
//! border) renders on top of the shader output through iced's standard
//! layering. The overlay owns the hover state machine and routes events to
//! the project's real button widgets so the play, heart, and checkbox keep
//! their canonical hover / press affordances.

use std::borrow::Cow;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, LazyLock};
use std::time::Instant;

use iced::advanced::renderer::{Quad, Style};
use iced::advanced::text::{Renderer as TextRenderer, Text as AdvText};
use iced::advanced::widget::{Operation, Tree, tree};
use iced::advanced::{Clipboard, Layout, Renderer, Shell, Widget, layout};
use iced::widget::text::{Alignment as TextAlign, LineHeight, Shaping, Wrapping};
use iced::widget::{column, container, stack};
use iced::{
    Border,
    Color,
    Element,
    Event,
    Font,
    Length,
    Padding,
    Pixels,
    Point,
    Rectangle,
    Renderer as IcedRenderer,
    Size,
    Theme,
    alignment,
    mouse,
    window,
};

use super::util::paint_centered_icon;
use crate::animate::hover::{EPSILON, Hover};
use crate::widget::blur::Backdrop;
use crate::widget::text::{media_overlay, shape_widest};
use crate::widget::{blurred_image, button};
use crate::{border, color, font, spacing, style};

const TIME_PILL_H: f32 = 24.0;
const WATCHED_PILL_H: f32 = 28.0;
const PROGRESS_HEIGHT: f32 = 4.0;

/// Hover dim, blended in linear space. Skip `srgb_alpha` so the perceptual
/// darken matches the authored value.
const DIM_ALPHA: f32 = 0.55;

const PILL_LABEL_SIZE: f32 = 10.0;

const TIME_ICON_SIZE: f32 = 14.0;
const WATCHED_CHECK_SIZE: f32 = 20.0;
const HEART_DIAMETER: f32 = 36.0;

/// Inset of the checkbox from the pill's left edge. Centres the checkbox
/// inside the compact circle and stays put as the pill grows.
const WATCHED_CHECK_INSET: f32 = (WATCHED_PILL_H - WATCHED_CHECK_SIZE) * 0.5;

/// Shaped once per process since the label is constant.
static WATCHED_LABEL_WIDTH: LazyLock<f32> =
    LazyLock::new(|| measure_overlay_width("Watched"));

#[derive(Clone, Copy)]
pub(crate) enum Aspect {
    Landscape,
    Portrait,
    Square,
}

pub(crate) struct CardFrame<'a, Message> {
    pub(crate) backdrop: Backdrop,
    #[allow(dead_code)]
    pub(crate) aspect: Aspect,
    pub(crate) image_size: Size,
    pub(crate) corner_radius: f32,
    pub(crate) play_size: f32,
    pub(crate) watched: bool,
    pub(crate) favourite: bool,
    pub(crate) progress: Option<f32>,
    pub(crate) time_left: Option<Cow<'static, str>>,
    pub(crate) label: Option<Element<'a, Message>>,
    pub(crate) subtext: Option<Element<'a, Message>>,
    pub(crate) overlay: Option<Element<'a, Message>>,
    pub(crate) on_press: Option<Message>,
    pub(crate) on_play: Option<Message>,
    pub(crate) on_watched_toggled: Option<Box<dyn Fn(bool) -> Message + 'a>>,
    pub(crate) on_favourite_toggled: Option<Box<dyn Fn(bool) -> Message + 'a>>,
}

pub(crate) fn build<'a, Message>(card: CardFrame<'a, Message>) -> Element<'a, Message>
where
    Message: Clone + 'a,
{
    let CardFrame {
        backdrop,
        aspect: _,
        image_size,
        corner_radius,
        play_size,
        watched,
        favourite,
        progress,
        time_left,
        label,
        subtext,
        overlay,
        on_press,
        on_play,
        on_watched_toggled,
        on_favourite_toggled,
    } = card;

    let time_label_width = time_left
        .as_deref()
        .map(measure_overlay_width)
        .unwrap_or(0.0);
    let watched_label_width = *WATCHED_LABEL_WIDTH;

    let time_pill_width = if time_left.is_some() {
        (spacing::PAD_10 * 2.0) + TIME_ICON_SIZE + spacing::GAP_6 + time_label_width
    } else {
        0.0
    };

    let watched_pill_compact = WATCHED_PILL_H;
    let watched_pill_expanded = WATCHED_CHECK_INSET
        + WATCHED_CHECK_SIZE
        + spacing::GAP_8
        + watched_label_width
        + spacing::GAP_8;

    // Packs hover and watched factors so the regions closure sees a
    // consistent pair across one atomic load.
    let shared_factors = Arc::new(AtomicFactors::new());
    let has_watched_chrome = on_watched_toggled.is_some();

    let image: Element<'a, Message> = {
        let shared_factors = Arc::clone(&shared_factors);
        let has_progress = progress.is_some();

        let mut builder = blurred_image::blurred_image(backdrop)
            .corner_radius(corner_radius)
            .width(Length::Fixed(image_size.width))
            .height(Length::Fixed(image_size.height))
            .regions_fn(move |size| {
                let (t_hover, t_watched) = shared_factors.load();
                regions_for(
                    size,
                    t_hover,
                    t_watched,
                    watched,
                    has_watched_chrome,
                    has_progress,
                    play_size,
                    time_pill_width,
                    watched_pill_compact,
                    watched_pill_expanded,
                )
            });

        if let Some(fill) = progress {
            builder = builder.progress_strip(
                fill.clamp(0.0, 1.0),
                PROGRESS_HEIGHT,
                color::primary(),
                color::with_alpha(Color::BLACK, color::srgb_alpha(0.45)),
            );
        }

        builder.into()
    };

    let play_button: Option<Element<'a, Message>> = on_play.as_ref().map(|msg| {
        let glyph = if watched { "replay" } else { "play_arrow" };
        button::accent(glyph, button::AccentSizeVariant::Main, msg.clone())
    });

    let heart_button: Option<Element<'a, Message>> =
        on_favourite_toggled.as_ref().map(|emit| {
            button::icon(
                "favorite",
                button::IconSizeVariant::Main,
                favourite,
                emit(!favourite),
            )
        });

    // Shared so the pill background and the checkbox child publish the
    // same value when toggled.
    let watched_toggle: Option<Message> =
        on_watched_toggled.as_ref().map(|emit| emit(!watched));
    let watched_checkbox: Option<Element<'a, Message>> =
        watched_toggle.clone().map(|msg| {
            button::checkbox(watched, button::CheckboxSizeVariant::Alt, Some(msg))
        });

    let mut children: Vec<Element<'a, Message>> = Vec::with_capacity(3);

    let play_idx = play_button.map(|btn| {
        children.push(btn);
        children.len() - 1
    });

    let heart_idx = heart_button.map(|btn| {
        children.push(btn);
        children.len() - 1
    });

    let watched_idx = watched_checkbox.map(|btn| {
        children.push(btn);
        children.len() - 1
    });

    let chrome = Element::new(ChromeOverlay {
        image_size,
        corner_radius,
        play_size,
        watched,
        has_progress: progress.is_some(),
        time_left,
        time_pill_width,
        watched_pill_compact,
        watched_pill_expanded,
        on_press,
        watched_toggle,
        shared_factors,
        children,
        play_idx,
        heart_idx,
        watched_idx,
    });

    let image_layer: Element<'a, Message> = match overlay {
        Some(overlay) => stack![image, chrome, overlay].into(),
        None => stack![image, chrome].into(),
    };

    // The padded surface gives the hover shadow room to fall outside the
    // image without being clipped by neighbouring cards.
    let padded_surface = container(image_layer).padding(Padding::new(spacing::PAD_2));
    let mut outer = column![padded_surface].spacing(spacing::GAP_8 - spacing::PAD_2);

    if label.is_some() || subtext.is_some() {
        let mut captions = column![].spacing(spacing::GAP_2);

        if let Some(label) = label {
            captions = captions.push(label);
        }
        if let Some(subtext) = subtext {
            captions = captions.push(subtext);
        }

        outer = outer.push(captions);
    }

    outer.into()
}

/// Blurred-region geometry handed to the composite shader. Mirrors what
/// the chrome overlay paints over the top. Every region is a pill so the
/// shader clamps each corner to its rect's smallest half-extent.
#[allow(clippy::too_many_arguments)]
fn regions_for(
    size: Size,
    t_hover: f32,
    t_watched: f32,
    watched: bool,
    has_watched_chrome: bool,
    has_progress: bool,
    play_size: f32,
    time_pill_width: f32,
    watched_pill_compact: f32,
    watched_pill_expanded: f32,
) -> Vec<blurred_image::BlurRegion> {
    let mut out = Vec::with_capacity(4);

    if has_progress && time_pill_width > 0.0 {
        out.push(blurred_image::BlurRegion::pill(Rectangle::new(
            Point::new(spacing::PAD_12, size.height - spacing::PAD_12 - TIME_PILL_H),
            Size::new(time_pill_width, TIME_PILL_H),
        )));
    }

    if t_hover > EPSILON {
        // Play scales from the image centre so the frosted backdrop stays
        // concentric with the button.
        let scaled = play_size * t_hover;
        out.push(blurred_image::BlurRegion::pill(Rectangle::new(
            Point::new((size.width - scaled) * 0.5, (size.height - scaled) * 0.5),
            Size::new(scaled, scaled),
        )));

        // Heart scales from its rest centre, not from the image corner,
        // so the frost stays under the button throughout the animation.
        let heart_cx = size.width - spacing::PAD_12 - HEART_DIAMETER * 0.5;
        let heart_cy = size.height - spacing::PAD_12 - HEART_DIAMETER * 0.5;
        let heart = HEART_DIAMETER * t_hover;

        out.push(blurred_image::BlurRegion::pill(Rectangle::new(
            Point::new(heart_cx - heart * 0.5, heart_cy - heart * 0.5),
            Size::new(heart, heart),
        )));
    }

    // The pill is right-anchored at the top-right of the image. It grows
    // leftward on hover. Skip it when there is no chrome to paint over so
    // the frosted patch never leaks past the image edge.
    let watched_visible =
        has_watched_chrome && has_visible_watched_pill(watched, t_watched);

    if watched_visible {
        let compact = if watched { watched_pill_compact } else { 0.0 };
        let width = compact + (watched_pill_expanded - compact) * t_watched;
        let pill_right = size.width - spacing::PAD_12;

        out.push(blurred_image::BlurRegion::pill(Rectangle::new(
            Point::new(pill_right - width, spacing::PAD_12),
            Size::new(width, WATCHED_PILL_H),
        )));
    }

    out
}

/// Chrome layer drawn on top of the blurred image via stack. Owns the
/// hover state and hosts the project's button widgets as children.
struct ChromeOverlay<'a, Message> {
    image_size: Size,
    corner_radius: f32,
    play_size: f32,
    watched: bool,
    has_progress: bool,
    time_left: Option<Cow<'static, str>>,
    time_pill_width: f32,
    watched_pill_compact: f32,
    watched_pill_expanded: f32,
    on_press: Option<Message>,
    /// Pre-computed toggle message so a click anywhere on the pill
    /// publishes the same value as a direct click on the checkbox child.
    watched_toggle: Option<Message>,
    /// Hover and watched-pill factors packed into one atomic for the
    /// regions closure.
    shared_factors: Arc<AtomicFactors>,
    children: Vec<Element<'a, Message>>,
    play_idx: Option<usize>,
    heart_idx: Option<usize>,
    watched_idx: Option<usize>,
}

#[derive(Default, Clone, Copy)]
struct State {
    hover: Hover,
    watched_pill: Hover,
    surface_press: bool,
    watched_press: bool,
}

impl<'a, Message> ChromeOverlay<'a, Message>
where
    Message: Clone + 'a,
{
    fn play_origin(&self, image: Rectangle) -> Point {
        Point::new(
            image.x + (image.width - self.play_size) * 0.5,
            image.y + (image.height - self.play_size) * 0.5,
        )
    }

    fn heart_origin(&self, image: Rectangle) -> Point {
        Point::new(
            image.x + image.width - spacing::PAD_12 - HEART_DIAMETER,
            image.y + image.height - spacing::PAD_12 - HEART_DIAMETER,
        )
    }

    fn watched_pill_width(&self, factor: f32) -> f32 {
        let compact = if self.watched {
            self.watched_pill_compact
        } else {
            0.0
        };
        compact + (self.watched_pill_expanded - compact) * factor
    }

    /// Top-left of the watched checkbox child. Slides left with the pill
    /// so the box stays at the same inset throughout the animation.
    fn watched_checkbox_origin(&self, image: Rectangle, factor: f32) -> Point {
        let pill_right = image.x + image.width - spacing::PAD_12;
        let width = self.watched_pill_width(factor);
        let pill_left = pill_right - width;

        Point::new(
            pill_left + WATCHED_CHECK_INSET,
            image.y + spacing::PAD_12 + WATCHED_CHECK_INSET,
        )
    }

    fn watched_pill_bounds(&self, image: Rectangle, factor: f32) -> Rectangle {
        let pill_right = image.x + image.width - spacing::PAD_12;
        let width = self.watched_pill_width(factor);

        Rectangle::new(
            Point::new(pill_right - width, image.y + spacing::PAD_12),
            Size::new(width, WATCHED_PILL_H),
        )
    }

    /// Live iff the card wires up an `on_watched_toggled` callback AND
    /// the pill has something to render. Folds both halves into one place
    /// so the draw, update, and mouse_interaction paths cannot drift.
    fn watched_pill_visible(&self, factor: f32) -> bool {
        self.watched_idx.is_some() && has_visible_watched_pill(self.watched, factor)
    }
}

impl<'a, Message> Widget<Message, Theme, IcedRenderer> for ChromeOverlay<'a, Message>
where
    Message: Clone + 'a,
{
    fn size(&self) -> Size<Length> {
        Size {
            width: Length::Fill,
            height: Length::Fill,
        }
    }

    fn layout(
        &mut self,
        tree: &mut Tree,
        renderer: &IcedRenderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        let max = limits.max();
        let bounds = Size::new(
            max.width.min(self.image_size.width),
            max.height.min(self.image_size.height),
        );
        let image = Rectangle::new(Point::ORIGIN, bounds);

        // Read the live factor from tree::State so the checkbox origin
        // tracks the pill expansion. `update` invalidates layout each
        // animation frame so this re-runs.
        let now = Instant::now();
        let state = tree.state.downcast_ref::<State>();
        let watched_factor = state.watched_pill.current(now);

        let play_origin = self.play_origin(image);
        let heart_origin = self.heart_origin(image);
        let watched_origin = self.watched_checkbox_origin(image, watched_factor);

        let mut child_nodes = Vec::with_capacity(self.children.len());
        for (idx, (child, child_tree)) in self
            .children
            .iter_mut()
            .zip(tree.children.iter_mut())
            .enumerate()
        {
            let origin = if Some(idx) == self.play_idx {
                play_origin
            } else if Some(idx) == self.heart_idx {
                heart_origin
            } else if Some(idx) == self.watched_idx {
                watched_origin
            } else {
                Point::ORIGIN
            };

            let child_limits =
                layout::Limits::new(Size::ZERO, Size::new(f32::INFINITY, f32::INFINITY));
            let node = child
                .as_widget_mut()
                .layout(child_tree, renderer, &child_limits)
                .move_to(origin);

            child_nodes.push(node);
        }

        layout::Node::with_children(bounds, child_nodes)
    }

    fn draw(
        &self,
        tree: &Tree,
        renderer: &mut IcedRenderer,
        theme: &Theme,
        style: &Style,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
    ) {
        let now = Instant::now();
        let state = tree.state.downcast_ref::<State>();
        let factor = state.hover.current(now);
        let watched_factor = state.watched_pill.current(now);
        let image = layout.bounds();

        // Drop shadow on a transparent quad. Only the cast colour reads.
        if factor > EPSILON {
            renderer.fill_quad(
                Quad {
                    bounds: image,
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

        // Hover dim. See `DIM_ALPHA` for the linear-space caveat.
        if factor > EPSILON {
            renderer.fill_quad(
                Quad {
                    bounds: image,
                    border: Border {
                        radius: self.corner_radius.into(),
                        ..Border::default()
                    },
                    ..Quad::default()
                },
                color::with_alpha(Color::BLACK, DIM_ALPHA * factor),
            );
        }

        if self.has_progress && self.time_pill_width > 0.0 {
            self.paint_time_pill_at(renderer, image);
        }

        let watched_visible = self.watched_pill_visible(watched_factor);
        if watched_visible {
            self.draw_watched_pill_background(renderer, image, watched_factor);
        }

        // Children render after the pill backdrops so the icons sit on
        // top. Hover-only chrome stays hidden until the factor lifts.
        let mut child_layouts = layout.children();

        for (idx, (child, child_tree)) in
            self.children.iter().zip(tree.children.iter()).enumerate()
        {
            let child_layout = child_layouts.next().expect("child layout");

            let hover_only = Some(idx) == self.play_idx || Some(idx) == self.heart_idx;
            if hover_only && factor < EPSILON {
                continue;
            }

            if Some(idx) == self.watched_idx && !watched_visible {
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
        }

        // The label fades in once the pill is wide enough to hold it.
        if watched_visible {
            let label_alpha = smoothstep(0.4, 1.0, watched_factor);
            if label_alpha > EPSILON {
                let pill = self.watched_pill_bounds(image, watched_factor);
                let label_left =
                    pill.x + WATCHED_CHECK_INSET + WATCHED_CHECK_SIZE + spacing::GAP_8;
                let label_right = pill.x + pill.width - spacing::GAP_8;
                let label_area = Rectangle::new(
                    Point::new(label_left, pill.y),
                    Size::new((label_right - label_left).max(0.0), pill.height),
                );

                paint_label_in(
                    renderer,
                    "Watched",
                    label_area,
                    PILL_LABEL_SIZE,
                    font::bold(),
                    color::with_alpha(color::TEXT_PRIMARY, label_alpha),
                    TextAlign::Left,
                );
            }
        }

        // Accent border last so it sits over the chrome and dim.
        if factor > EPSILON {
            renderer.fill_quad(
                Quad {
                    bounds: image,
                    border: Border {
                        color: color::primary(),
                        width: style::BORDER_WIDTH * factor,
                        radius: self.corner_radius.into(),
                    },
                    ..Quad::default()
                },
                Color::TRANSPARENT,
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
        renderer: &IcedRenderer,
        operation: &mut dyn Operation,
    ) {
        for ((child, child_tree), child_layout) in self
            .children
            .iter_mut()
            .zip(tree.children.iter_mut())
            .zip(layout.children())
        {
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
        let now = Instant::now();
        let image_bounds = layout.bounds();
        let over_image = cursor.is_over(image_bounds);

        let state_before = tree.state.downcast_ref::<State>();
        let factor_before = state_before.hover.current(now);
        let watched_before = state_before.watched_pill.current(now);

        // Press latches snapshotted before any child captures the release,
        // so the release arm can still see the pre-cleared values.
        let watched_press_before = state_before.watched_press;
        let surface_press_before = state_before.surface_press;
        let watched_visible_before = self.watched_pill_visible(watched_before);

        // Forward to children. Hover-only chrome stays inert until the
        // factor lifts so an invisible button cannot eat a surface click.
        let mut child_layouts = layout.children();

        for (idx, (child, child_tree)) in self
            .children
            .iter_mut()
            .zip(tree.children.iter_mut())
            .enumerate()
        {
            let child_layout = child_layouts.next().expect("child layout");

            let alive = if Some(idx) == self.play_idx || Some(idx) == self.heart_idx {
                factor_before > EPSILON
            } else if Some(idx) == self.watched_idx {
                watched_visible_before
            } else {
                true
            };

            if alive {
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
        }

        let state = tree.state.downcast_mut::<State>();
        let hover_changed = state.hover.flip(over_image, now);
        let watched_changed = state.watched_pill.flip(over_image, now);

        // One atomic store keeps the regions closure seeing a consistent pair.
        let hover_factor = state.hover.current(now);
        let watched_factor = state.watched_pill.current(now);
        self.shared_factors.store(hover_factor, watched_factor);

        if hover_changed || watched_changed {
            shell.request_redraw();
            shell.invalidate_layout();
        }

        // Clear the latches on any Left release before the captured-event
        // bailout. Otherwise a child capturing the release leaves them hot.
        if matches!(
            event,
            Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left))
        ) {
            state.watched_press = false;
            state.surface_press = false;
        }

        if shell.is_event_captured() {
            return;
        }

        // Whole-pill click toggles watched. Hit-test against the live
        // pill bounds so the target tracks the expansion.
        let watched_pill_target = self.watched_toggle.is_some()
            && watched_visible_before
            && cursor
                .position()
                .map(|p| {
                    self.watched_pill_bounds(image_bounds, watched_before)
                        .contains(p)
                })
                .unwrap_or(false);

        let surface_press_target =
            self.on_press.is_some() && over_image && !watched_pill_target;

        match event {
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left))
                if watched_pill_target =>
            {
                state.watched_press = true;
            },
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left))
                if surface_press_target =>
            {
                state.surface_press = true;
            },
            Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)) => {
                if watched_press_before
                    && watched_pill_target
                    && let Some(message) = self.watched_toggle.clone()
                {
                    shell.publish(message);
                    shell.capture_event();
                } else if surface_press_before
                    && over_image
                    && let Some(message) = self.on_press.clone()
                {
                    shell.publish(message);
                    shell.capture_event();
                }
            },
            Event::Window(window::Event::RedrawRequested(_))
                if state.hover.animating(now) || state.watched_pill.animating(now) =>
            {
                shell.request_redraw();
                shell.invalidate_layout();
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
        renderer: &IcedRenderer,
    ) -> mouse::Interaction {
        let now = Instant::now();
        let state = tree.state.downcast_ref::<State>();
        let factor = state.hover.current(now);
        let watched_factor = state.watched_pill.current(now);
        let watched_visible = self.watched_pill_visible(watched_factor);

        let mut child_layouts = layout.children();
        for (idx, (child, child_tree)) in
            self.children.iter().zip(tree.children.iter()).enumerate()
        {
            let child_layout = child_layouts.next().expect("child layout");
            let alive = if Some(idx) == self.play_idx || Some(idx) == self.heart_idx {
                factor > EPSILON
            } else if Some(idx) == self.watched_idx {
                watched_visible
            } else {
                true
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

        if self.on_press.is_some() && cursor.is_over(layout.bounds()) {
            mouse::Interaction::Pointer
        } else {
            mouse::Interaction::None
        }
    }
}

impl<'a, Message> ChromeOverlay<'a, Message>
where
    Message: Clone + 'a,
{
    fn draw_watched_pill_background(
        &self,
        renderer: &mut IcedRenderer,
        image: Rectangle,
        factor: f32,
    ) {
        let pill = self.watched_pill_bounds(image, factor);
        if pill.width < EPSILON {
            return;
        }

        paint_pill_background(renderer, pill);
    }

    fn paint_time_pill_at(&self, renderer: &mut IcedRenderer, image: Rectangle) {
        let pill = Rectangle::new(
            Point::new(
                image.x + spacing::PAD_12,
                image.y + image.height - spacing::PAD_12 - TIME_PILL_H,
            ),
            Size::new(self.time_pill_width, TIME_PILL_H),
        );
        paint_pill_background(renderer, pill);

        let Some(label) = self.time_left.as_deref() else {
            return;
        };

        let icon_area = Rectangle::new(
            Point::new(pill.x + spacing::PAD_10, pill.y),
            Size::new(TIME_ICON_SIZE, pill.height),
        );
        paint_centered_icon(
            renderer,
            "access_time",
            icon_area,
            TIME_ICON_SIZE,
            color::TEXT_PRIMARY,
        );

        let label_left = pill.x + spacing::PAD_10 + TIME_ICON_SIZE + spacing::GAP_6;
        let label_area = Rectangle::new(
            Point::new(label_left, pill.y),
            Size::new(
                (pill.x + pill.width - spacing::PAD_10 - label_left).max(0.0),
                pill.height,
            ),
        );
        paint_label_in(
            renderer,
            label,
            label_area,
            PILL_LABEL_SIZE,
            font::bold(),
            color::TEXT_PRIMARY,
            TextAlign::Left,
        );
    }
}

fn paint_pill_background(renderer: &mut IcedRenderer, bounds: Rectangle) {
    if bounds.width <= 0.0 {
        return;
    }
    renderer.fill_quad(
        Quad {
            bounds,
            border: Border {
                radius: (bounds.height * 0.5).into(),
                ..Border::default()
            },
            ..Quad::default()
        },
        color::knob_fill_off(),
    );
}

#[allow(clippy::too_many_arguments)]
fn paint_label_in(
    renderer: &mut IcedRenderer,
    content: &str,
    area: Rectangle,
    glyph_size: f32,
    font: Font,
    color: Color,
    align_x: TextAlign,
) {
    if area.width < 1.0 {
        return;
    }

    let anchor_x = match align_x {
        TextAlign::Left | TextAlign::Default | TextAlign::Justified => area.x,
        TextAlign::Center => area.x + area.width * 0.5,
        TextAlign::Right => area.x + area.width,
    };
    let anchor = Point::new(anchor_x, area.y + area.height * 0.5);

    let text = AdvText {
        content: content.to_string(),
        // Infinite layout width so a narrow `area` during animation cannot
        // wrap the label. Overflow is hidden by the clip rect.
        bounds: Size::new(f32::INFINITY, area.height),
        size: Pixels(glyph_size),
        line_height: LineHeight::Relative(1.0),
        font,
        align_x,
        align_y: alignment::Vertical::Center,
        shaping: Shaping::Advanced,
        wrapping: Wrapping::None,
    };

    TextRenderer::fill_text(renderer, text, anchor, color, area);
}

/// Live when the card is already watched, or the hover has lifted off
/// zero. A card with no watched chrome must paint nothing past the edge.
fn has_visible_watched_pill(watched: bool, factor: f32) -> bool {
    watched || factor > EPSILON
}

/// Hover and watched-pill factors packed into one atomic so the regions
/// closure sees a consistent pair across a single load.
#[derive(Debug, Default)]
struct AtomicFactors(AtomicU64);

impl AtomicFactors {
    fn new() -> Self {
        Self(AtomicU64::new(0))
    }

    fn store(&self, hover: f32, watched: f32) {
        let packed = (hover.to_bits() as u64) << 32 | watched.to_bits() as u64;
        self.0.store(packed, Ordering::Relaxed);
    }

    fn load(&self) -> (f32, f32) {
        let packed = self.0.load(Ordering::Relaxed);
        let hover = f32::from_bits((packed >> 32) as u32);
        let watched = f32::from_bits(packed as u32);
        (hover, watched)
    }
}

fn measure_overlay_width(content: &str) -> f32 {
    // Shape via the project's typography helper so the measurement matches
    // what the pill label renders as.
    let text = media_overlay(content.to_owned()).color(color::TEXT_PRIMARY);
    shape_widest(std::iter::once(&text))
}

fn smoothstep(edge0: f32, edge1: f32, x: f32) -> f32 {
    let t = ((x - edge0) / (edge1 - edge0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// Skeleton placeholder matching the chassis layout (image area + two
/// caption rows). Each per-card builder calls this with its own image dims
/// and corner radius so a loading page holds its shape.
pub(crate) fn skeleton<'a, Message>(
    image_size: Size,
    corner_radius: f32,
) -> Element<'a, Message>
where
    Message: 'a,
{
    use crate::widget::skeleton::skeleton as shimmer;

    let inner = (image_size.width - spacing::PAD_6 * 2.0).max(0.0);

    let image: Element<'a, Message> = shimmer()
        .width(Length::Fixed(image_size.width))
        .height(Length::Fixed(image_size.height))
        .radius(corner_radius)
        .into();

    let label: Element<'a, Message> = shimmer()
        .width(Length::Fixed(inner * SKELETON_LABEL_FRAC))
        .height(Length::Fixed(SKELETON_LABEL_H))
        .radius(border::ROUNDED_XS)
        .into();

    let subtext: Element<'a, Message> = shimmer()
        .width(Length::Fixed(inner * SKELETON_SUBTEXT_FRAC))
        .height(Length::Fixed(SKELETON_SUBTEXT_H))
        .radius(border::ROUNDED_XS)
        .into();

    let captions = container(column![label, subtext].spacing(spacing::GAP_8))
        .padding(Padding::default().horizontal(spacing::PAD_6));

    let padded_image = container(image).padding(Padding::new(spacing::PAD_2));

    column![padded_image, captions]
        .spacing(spacing::GAP_8 - spacing::PAD_2)
        .into()
}

const SKELETON_LABEL_H: f32 = 13.0;
const SKELETON_SUBTEXT_H: f32 = 11.0;
const SKELETON_LABEL_FRAC: f32 = 0.75;
const SKELETON_SUBTEXT_FRAC: f32 = 0.55;
