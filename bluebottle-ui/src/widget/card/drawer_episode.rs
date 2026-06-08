//! Drawer Episode Row. The list unit of the slide-in drawer's Episodes tab.
//! A 120x68 thumbnail leads, with chrome layered on hover (40% scrim and
//! centred play) and on the persistent states (4px accent progress strip,
//! watched dim-down with the bordered-glass checkbox stamped over the still).
//! The identity column carries an `EP NN` eyebrow, the episode title as a
//! link, and a dotted meta line that picks up an accent `N% watched` chip
//! for in-progress rows. A trailing flat more button publishes
//! `on_expand_click`. Any press over the rest of the row publishes
//! `on_click`.
//!
//! Built as one custom widget so the row-wide hover signal can drive both
//! the chassis tint and the thumbnail chrome together. Hovering anywhere
//! outside the more button lifts the bg to white at 4% and reveals the play
//! disc.

use std::borrow::Cow;
use std::time::Instant;

use iced::Center;
use iced::advanced::renderer::{Quad, Style};
use iced::advanced::widget::{Operation, Tree, tree};
use iced::advanced::{Clipboard, Layout, Renderer, Shell, Widget, layout};
use iced::widget::{column, container, image, row};
use iced::{
    Border,
    Color,
    Element,
    Event,
    Length,
    Padding,
    Point,
    Rectangle,
    Renderer as IcedRenderer,
    Size,
    Theme,
    mouse,
};

use super::util::paint_centered_icon;
use crate::animate::hover::{EPSILON, Hover};
use crate::widget::text;
use crate::{color, font};

const ROW_RADIUS: f32 = 10.0;
const ROW_PAD_V: f32 = 10.0;
const ROW_PAD_H: f32 = 12.0;
const ROW_GAP: f32 = 12.0;
const TEXT_GAP: f32 = 4.0;
const META_GAP: f32 = 6.0;
const EYEBROW_GAP: f32 = 6.0;

const THUMB_W: f32 = 120.0;
const THUMB_H: f32 = 68.0;
const THUMB_RADIUS: f32 = 8.0;

const PROGRESS_HEIGHT: f32 = 4.0;
const CHECKBOX_INSET: f32 = 4.0;
const CHECKBOX_SIZE: f32 = 20.0;

const PLAY_SIZE: f32 = 36.0;
const PLAY_GLYPH_SIZE: f32 = 20.0;
const SCRIM_ALPHA: f32 = 0.40;
const WATCHED_OPACITY: f32 = 0.6;
const WATCHED_GREY_ALPHA: f32 = 0.45;
const WATCHED_ROW_WASH_ALPHA: f32 = 0.22;
const PLAY_DISC_ALPHA: f32 = 0.18;

const MORE_DIAMETER: f32 = 32.0;
const MORE_GLYPH: f32 = 16.0;

/// Width to fall back to when the parent gives an unbounded width limit.
/// Without it the identity column would try to shape its title on a single
/// arbitrarily long line.
const UNBOUNDED_WIDTH_FALLBACK: f32 = 480.0;

const META_TEXT_SIZE: f32 = 10.0;

const IDENTITY_IDX: usize = 0;
const MORE_IDX: usize = 1;
const CHECKBOX_IDX: usize = 2;

/// Creates a Drawer Episode Row.
pub fn drawer_episode_row<Message>(
    thumbnail: image::Handle,
    episode_number: u32,
    title: impl Into<Cow<'static, str>>,
) -> DrawerEpisodeRow<Message>
where
    Message: Clone,
{
    DrawerEpisodeRow {
        thumbnail,
        episode_number,
        title: title.into(),
        meta: Vec::new(),
        watched: false,
        progress: None,
        on_click: None,
        on_expand_click: None,
    }
}

/// Builder for [`drawer_episode_row`].
pub struct DrawerEpisodeRow<Message> {
    thumbnail: image::Handle,
    episode_number: u32,
    title: Cow<'static, str>,
    meta: Vec<Cow<'static, str>>,
    watched: bool,
    progress: Option<f32>,
    on_click: Option<Message>,
    on_expand_click: Option<Message>,
}

impl<Message> DrawerEpisodeRow<Message>
where
    Message: Clone,
{
    /// Meta line items. Rendered with [`color::TEXT_DARK`] dot separators.
    pub fn meta(
        mut self,
        items: impl IntoIterator<Item = impl Into<Cow<'static, str>>>,
    ) -> Self {
        self.meta = items.into_iter().map(Into::into).collect();
        self
    }

    /// Marks the row as watched. Dims the row, greys the still, and stamps
    /// the bordered-glass checkbox over the top-right of the thumbnail. A
    /// watched row also drops any [`progress`](Self::progress) chrome so the
    /// two states never claim the same row at once.
    pub fn watched(mut self, watched: bool) -> Self {
        self.watched = watched;
        self
    }

    /// In-progress state. `progress` is in `[0, 1]`. Renders the accent strip
    /// across the bottom of the thumbnail and appends an accent `N% watched`
    /// chip to the meta row. Ignored when [`watched`](Self::watched) is set.
    pub fn progress(mut self, progress: f32) -> Self {
        self.progress = Some(progress.clamp(0.0, 1.0));
        self
    }

    /// Press anywhere on the row outside the more button and the title link.
    pub fn on_click(mut self, message: Message) -> Self {
        self.on_click = Some(message);
        self
    }

    /// Press on the trailing more button or the title link.
    pub fn on_expand_click(mut self, message: Message) -> Self {
        self.on_expand_click = Some(message);
        self
    }
}

impl<'a, Message> From<DrawerEpisodeRow<Message>> for Element<'a, Message>
where
    Message: Clone + 'a,
{
    fn from(row_def: DrawerEpisodeRow<Message>) -> Self {
        let DrawerEpisodeRow {
            thumbnail,
            episode_number,
            title,
            meta,
            watched,
            progress,
            on_click,
            on_expand_click,
        } = row_def;

        // Watched takes precedence so a single row never advertises both
        // completed and in-progress chrome.
        let effective_progress = if watched { None } else { progress };

        let opacity_factor = if watched { WATCHED_OPACITY } else { 1.0 };
        let accent = if watched {
            color::TEXT_SECONDARY
        } else {
            color::primary()
        };
        let primary = color::with_alpha(color::TEXT_PRIMARY, opacity_factor);
        let secondary = color::with_alpha(color::TEXT_SECONDARY, opacity_factor);
        let separator = color::with_alpha(color::TEXT_DARK, opacity_factor);

        let eyebrow = text::eyebrow(format!("EP {episode_number:02}"), text::Variant::Main)
            .font(font::bold())
            .color(accent);

        let title_text = text::card_title(title)
            .font(font::semibold())
            .color(primary);

        let title_widget: Element<'a, Message> = match on_expand_click.clone() {
            Some(message) => crate::widget::link::link(title_text, message).into(),
            None => title_text.into(),
        };

        let title_row = row![eyebrow, title_widget]
            .spacing(EYEBROW_GAP)
            .align_y(Center);

        let mut meta_row = row![].spacing(META_GAP).align_y(Center);
        let mut pushed = 0usize;

        for item in meta {
            if pushed > 0 {
                meta_row = meta_row.push(meta_separator(separator));
            }
            meta_row = meta_row.push(meta_item(item, secondary));
            pushed += 1;
        }

        // Drop the chip when the rounded percentage would read as zero so
        // 'started but no progress recorded' rows do not advertise a false
        // 0% claim.
        if let Some(p) = effective_progress {
            let percent = (p * 100.0).round() as u32;
            if percent > 0 {
                if pushed > 0 {
                    meta_row = meta_row.push(meta_separator(separator));
                }
                meta_row = meta_row.push(
                    iced::widget::text(format!("{percent}% watched"))
                        .size(META_TEXT_SIZE)
                        .font(font::semibold())
                        .color(color::with_alpha(color::primary(), opacity_factor)),
                );
            }
        }

        let identity: Element<'a, Message> = column![title_row, meta_row]
            .spacing(TEXT_GAP)
            .width(Length::Fill)
            .into();

        let more: Element<'a, Message> =
            crate::widget::button::icon_flat("more_horiz", false, on_expand_click)
                .size(MORE_DIAMETER, MORE_GLYPH)
                .into();

        let mut children: Vec<Element<'a, Message>> = vec![identity, more];
        if watched {
            children.push(crate::widget::button::checkbox(
                true,
                crate::widget::button::CheckboxSizeVariant::Alt,
                None,
            ));
        }

        DrawerEpisodeRowWidget {
            thumbnail,
            watched,
            progress: effective_progress,
            children,
            on_click,
        }
        .into()
    }
}

/// Builds the dot separator. Static input lets iced borrow the str instead
/// of allocating per item per frame.
fn meta_separator<'a>(color: Color) -> iced::widget::Text<'a> {
    iced::widget::text("·").size(META_TEXT_SIZE).color(color)
}

/// Wraps a meta line item without forcing an allocation on the borrowed-Cow
/// path. iced's `text` only accepts `&'a str` or `String`, so we dispatch on
/// the Cow variant to forward the static slice where we can.
fn meta_item<'a>(item: Cow<'static, str>, color: Color) -> iced::widget::Text<'a> {
    match item {
        Cow::Borrowed(s) => iced::widget::text(s),
        Cow::Owned(s) => iced::widget::text(s),
    }
    .size(META_TEXT_SIZE)
    .color(color)
}

/// Shimmer placeholder matching the row layout. A 120x68 thumbnail block
/// beside two stacked bars for the title and meta line, with a circular dot
/// standing in for the trailing more button. Drop in while the episode list
/// is still loading so the drawer holds its rhythm.
pub fn drawer_episode_row_skeleton<'a, Message>() -> Element<'a, Message>
where
    Message: 'a,
{
    use crate::widget::skeleton::skeleton as shimmer;

    let thumb: Element<'a, Message> = shimmer()
        .width(Length::Fixed(THUMB_W))
        .height(Length::Fixed(THUMB_H))
        .radius(THUMB_RADIUS)
        .into();

    let title_bar: Element<'a, Message> = shimmer()
        .width(Length::Fixed(240.0))
        .height(Length::Fixed(13.0))
        .radius(4.0)
        .into();

    let meta_bar: Element<'a, Message> = shimmer()
        .width(Length::Fixed(160.0))
        .height(Length::Fixed(10.0))
        .radius(4.0)
        .into();

    let identity = column![title_bar, meta_bar]
        .spacing(TEXT_GAP)
        .width(Length::Fill);

    let more_dot: Element<'a, Message> = shimmer()
        .width(Length::Fixed(MORE_DIAMETER))
        .height(Length::Fixed(MORE_DIAMETER))
        .radius(MORE_DIAMETER * 0.5)
        .into();

    container(
        row![thumb, identity, more_dot]
            .spacing(ROW_GAP)
            .align_y(Center)
            .width(Length::Fill),
    )
    .width(Length::Fill)
    .padding(
        Padding::default()
            .vertical(ROW_PAD_V)
            .horizontal(ROW_PAD_H),
    )
    .into()
}

/// Single widget for the whole row. Owns the hover state and lays out the
/// identity column and the trailing more button beside a drawn thumbnail.
/// `children` is `[identity, more]`, gaining a third checkbox entry when the
/// row is watched.
struct DrawerEpisodeRowWidget<'a, Message> {
    thumbnail: image::Handle,
    watched: bool,
    progress: Option<f32>,
    children: Vec<Element<'a, Message>>,
    on_click: Option<Message>,
}

#[derive(Default, Clone, Copy)]
struct RowState {
    hover: Hover,
    surface_press: bool,
}

impl<'a, Message> From<DrawerEpisodeRowWidget<'a, Message>> for Element<'a, Message>
where
    Message: Clone + 'a,
{
    fn from(value: DrawerEpisodeRowWidget<'a, Message>) -> Self {
        Element::new(value)
    }
}

impl<'a, Message> Widget<Message, Theme, IcedRenderer> for DrawerEpisodeRowWidget<'a, Message>
where
    Message: Clone + 'a,
{
    fn size(&self) -> Size<Length> {
        Size {
            width: Length::Fill,
            height: Length::Shrink,
        }
    }

    fn layout(
        &mut self,
        tree: &mut Tree,
        renderer: &IcedRenderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        let padding = Padding {
            top: ROW_PAD_V,
            right: ROW_PAD_H,
            bottom: ROW_PAD_V,
            left: ROW_PAD_H,
        };

        // Fall back to a sensible width when the parent leaves us
        // unconstrained so the identity column never shapes its title on a
        // single endlessly long line.
        let raw_max = limits.shrink(padding).max();
        let inner_w = if raw_max.width.is_finite() {
            raw_max.width
        } else {
            UNBOUNDED_WIDTH_FALLBACK
        };

        let more_limits = layout::Limits::new(
            Size::new(MORE_DIAMETER, MORE_DIAMETER),
            Size::new(MORE_DIAMETER, MORE_DIAMETER),
        );
        let more_node = self.children[MORE_IDX].as_widget_mut().layout(
            &mut tree.children[MORE_IDX],
            renderer,
            &more_limits,
        );

        let identity_max_w =
            (inner_w - THUMB_W - ROW_GAP - MORE_DIAMETER - ROW_GAP).max(0.0);
        let identity_limits = layout::Limits::new(
            Size::ZERO,
            Size::new(identity_max_w, f32::INFINITY),
        );
        let identity_node = self.children[IDENTITY_IDX].as_widget_mut().layout(
            &mut tree.children[IDENTITY_IDX],
            renderer,
            &identity_limits,
        );

        let row_h = THUMB_H.max(identity_node.size().height);

        let identity_y = padding.top + (row_h - identity_node.size().height) * 0.5;
        let more_y = padding.top + (row_h - MORE_DIAMETER) * 0.5;

        let identity_node = identity_node.move_to(Point::new(
            padding.left + THUMB_W + ROW_GAP,
            identity_y,
        ));
        let more_node = more_node.move_to(Point::new(
            padding.left + inner_w - MORE_DIAMETER,
            more_y,
        ));

        let total = Size::new(
            inner_w + padding.left + padding.right,
            row_h + padding.top + padding.bottom,
        );

        let mut child_nodes = vec![identity_node, more_node];

        if self.watched {
            let thumb_y = padding.top + (row_h - THUMB_H) * 0.5;
            let checkbox_limits = layout::Limits::new(
                Size::new(CHECKBOX_SIZE, CHECKBOX_SIZE),
                Size::new(CHECKBOX_SIZE, CHECKBOX_SIZE),
            );
            let checkbox_node = self.children[CHECKBOX_IDX].as_widget_mut().layout(
                &mut tree.children[CHECKBOX_IDX],
                renderer,
                &checkbox_limits,
            );

            let cx = padding.left + THUMB_W - CHECKBOX_INSET - CHECKBOX_SIZE;
            let cy = thumb_y + CHECKBOX_INSET;
            child_nodes.push(checkbox_node.move_to(Point::new(cx, cy)));
        }

        layout::Node::with_children(total, child_nodes)
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
        use iced::advanced::image::Renderer as ImageRenderer;

        let bounds = layout.bounds();
        let now = Instant::now();
        let state = tree.state.downcast_ref::<RowState>();
        let hover = state.hover.current(now);

        let row_border = Border {
            radius: ROW_RADIUS.into(),
            ..Border::default()
        };

        if hover > EPSILON {
            renderer.fill_quad(
                Quad {
                    bounds,
                    border: row_border,
                    ..Quad::default()
                },
                color::with_alpha(color::WHITE, color::srgb_alpha(0.04) * hover),
            );
        }

        // Whole-row dim wash for completed episodes. The checkbox child
        // redraws over the top of this so its accent ring stays at full read.
        if self.watched {
            renderer.fill_quad(
                Quad {
                    bounds,
                    border: row_border,
                    ..Quad::default()
                },
                color::with_alpha(Color::BLACK, color::srgb_alpha(WATCHED_ROW_WASH_ALPHA)),
            );
        }

        // Derived from the same padding math the layout uses so the draw and
        // hit-test paths stay in lockstep without smuggling extra state.
        let thumb_bounds = Rectangle::new(
            Point::new(
                bounds.x + ROW_PAD_H,
                bounds.y + ROW_PAD_V + (row_inner_h(bounds) - THUMB_H) * 0.5,
            ),
            Size::new(THUMB_W, THUMB_H),
        );

        let rounded = Border {
            radius: THUMB_RADIUS.into(),
            ..Border::default()
        };

        ImageRenderer::draw_image(
            renderer,
            iced::advanced::image::Image {
                handle: self.thumbnail.clone(),
                filter_method: iced::widget::image::FilterMethod::Linear,
                rotation: iced::Radians(0.0),
                border_radius: THUMB_RADIUS.into(),
                opacity: 1.0,
                snap: true,
            },
            thumb_bounds,
            thumb_bounds,
        );

        if self.watched {
            renderer.fill_quad(
                Quad {
                    bounds: thumb_bounds,
                    border: rounded,
                    ..Quad::default()
                },
                color::with_alpha(Color::BLACK, WATCHED_GREY_ALPHA),
            );
        }

        if hover > EPSILON {
            renderer.fill_quad(
                Quad {
                    bounds: thumb_bounds,
                    border: rounded,
                    ..Quad::default()
                },
                color::with_alpha(Color::BLACK, SCRIM_ALPHA * hover),
            );
        }

        // Progress strip in the media-card recipe so the two surfaces read
        // as siblings. The bottom corners follow the thumbnail mask so the
        // strip never overhangs the rounded edge.
        if let Some(progress) = self.progress {
            let track = Rectangle::new(
                Point::new(
                    thumb_bounds.x,
                    thumb_bounds.y + thumb_bounds.height - PROGRESS_HEIGHT,
                ),
                Size::new(thumb_bounds.width, PROGRESS_HEIGHT),
            );
            let strip_border = Border {
                radius: iced::border::Radius {
                    top_left: 0.0,
                    top_right: 0.0,
                    bottom_right: THUMB_RADIUS,
                    bottom_left: THUMB_RADIUS,
                },
                ..Border::default()
            };

            renderer.fill_quad(
                Quad {
                    bounds: track,
                    border: strip_border,
                    ..Quad::default()
                },
                color::with_alpha(Color::BLACK, color::srgb_alpha(0.45)),
            );

            let fill_w = thumb_bounds.width * progress;
            if fill_w > EPSILON {
                renderer.fill_quad(
                    Quad {
                        bounds: Rectangle::new(track.position(), Size::new(fill_w, PROGRESS_HEIGHT)),
                        border: strip_border,
                        ..Quad::default()
                    },
                    color::primary(),
                );
            }
        }

        if hover > EPSILON {
            let cx = thumb_bounds.x + (thumb_bounds.width - PLAY_SIZE) * 0.5;
            let cy = thumb_bounds.y + (thumb_bounds.height - PLAY_SIZE) * 0.5;
            let disc = Rectangle::new(Point::new(cx, cy), Size::new(PLAY_SIZE, PLAY_SIZE));

            renderer.fill_quad(
                Quad {
                    bounds: disc,
                    border: Border {
                        radius: (PLAY_SIZE * 0.5).into(),
                        ..Border::default()
                    },
                    ..Quad::default()
                },
                color::with_alpha(color::WHITE, PLAY_DISC_ALPHA * hover),
            );

            paint_centered_icon(
                renderer,
                "play_arrow",
                disc,
                PLAY_GLYPH_SIZE,
                color::with_alpha(color::TEXT_PRIMARY, hover),
            );
        }

        let mut child_layouts = layout.children();
        for (child, child_tree) in self.children.iter().zip(tree.children.iter()) {
            let child_layout = child_layouts.next().expect("child layout");
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
    }

    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<RowState>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(RowState::default())
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
        let mut child_layouts = layout.children();
        for (child, child_tree) in self.children.iter_mut().zip(tree.children.iter_mut()) {
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
        let bounds = layout.bounds();
        let now = Instant::now();

        let more_bounds = layout
            .children()
            .nth(MORE_IDX)
            .expect("more layout")
            .bounds();

        // Forward to children first so the more button and the title link
        // can claim their own press before the row chassis sees it.
        let mut child_layouts = layout.children();
        for (child, child_tree) in self.children.iter_mut().zip(tree.children.iter_mut()) {
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

        let over_row = cursor.is_over(bounds);
        let over_more = cursor.is_over(more_bounds);
        let hover_target = over_row && !over_more;

        let state = tree.state.downcast_mut::<RowState>();
        if state.hover.flip(hover_target, now) {
            shell.request_redraw();
        }

        // Keep the redraw chain alive while the hover factor is still
        // settling. The check runs before the captured-event early returns
        // so a child capturing a release cannot strand the fade mid-flight.
        if state.hover.animating(now) {
            shell.request_redraw();
        }

        if matches!(
            event,
            Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left))
        ) {
            // Clear the latch on any release so the next press starts fresh,
            // even when a child captured this one.
            let surface_press_before = state.surface_press;
            state.surface_press = false;

            if shell.is_event_captured() {
                return;
            }

            if surface_press_before
                && over_row
                && !over_more
                && let Some(message) = self.on_click.clone()
            {
                shell.publish(message);
                shell.capture_event();
                return;
            }
        }

        if shell.is_event_captured() {
            return;
        }

        if let Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) = event
            && self.on_click.is_some()
            && over_row
            && !over_more
        {
            state.surface_press = true;
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
        let mut child_layouts = layout.children();
        for (child, child_tree) in self.children.iter().zip(tree.children.iter()) {
            let child_layout = child_layouts.next().expect("child layout");
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

        if self.on_click.is_some() && cursor.is_over(layout.bounds()) {
            mouse::Interaction::Pointer
        } else {
            mouse::Interaction::None
        }
    }
}

/// Inner content height inside the row padding, derived from the row bounds
/// so the draw path stays in lockstep with the layout without an extra
/// state field.
fn row_inner_h(bounds: Rectangle) -> f32 {
    (bounds.height - ROW_PAD_V * 2.0).max(0.0)
}
