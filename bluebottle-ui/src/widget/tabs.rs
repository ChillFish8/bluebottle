//! A horizontal tab strip with an animated `primary()` underline.
//!
//! Each tab takes an arbitrary child element. The selected tab paints
//! its content in [`color::TEXT_PRIMARY`] with the underline beneath it.
//! Unselected tabs paint in [`color::TEXT_SECONDARY`] and ease toward
//! [`color::TEXT_PRIMARY`] on hover, matching the design system's 100 ms
//! [`Hover`](crate::animate::hover::Hover) convention. The underline
//! slides between tabs when [`Tabs::selected`] changes, with mid-flight
//! reversal supported.
//!
//! The colour shift rides on iced's `text_color` cascade. Children that
//! set an explicit `.color(...)` on their text or icons will ignore the
//! cascade and stay at that fixed colour. Leave the content's colour
//! unset to opt into the animation.
//!
//! Selected tabs are inert. No pointer cursor, no hover affordance, no
//! press dispatch. Clicking the active tab is a no-op.

use std::time::Instant;

use iced::advanced::renderer::{Quad, Style as RendererStyle};
use iced::advanced::widget::{Operation, Tree, tree};
use iced::advanced::{Clipboard, Layout, Renderer, Shell, Widget, layout};
use iced::{
    Border,
    Element,
    Event,
    Length,
    Padding,
    Point,
    Rectangle,
    Size,
    mouse,
    window,
};

use crate::animate::hover::{FADE, PressState};
use crate::{color, easing};

const UNDERLINE_THICKNESS: f32 = 2.0;
const UNDERLINE_RADIUS: f32 = UNDERLINE_THICKNESS / 2.0;
const DEFAULT_PADDING: Padding = Padding {
    top: 8.0,
    right: 16.0,
    bottom: 8.0,
    left: 16.0,
};

/// Creates a tab strip over `children` with `selected` highlighted.
/// `on_select` maps the clicked tab's index to a message. Clicking the
/// already-selected tab publishes nothing.
pub fn tabs<'a, Message>(
    children: impl IntoIterator<Item = impl Into<Element<'a, Message>>>,
    selected: usize,
    on_select: impl Fn(usize) -> Message + 'a,
) -> Tabs<'a, Message>
where
    Message: 'a,
{
    Tabs {
        tabs: children.into_iter().map(Into::into).collect(),
        selected,
        on_select: Box::new(on_select),
        padding: DEFAULT_PADDING,
        spacing: 0.0,
        width: Length::Shrink,
        height: Length::Shrink,
    }
}

/// A configurable tab strip, built by [`tabs`].
pub struct Tabs<'a, Message> {
    tabs: Vec<Element<'a, Message>>,
    selected: usize,
    on_select: Box<dyn Fn(usize) -> Message + 'a>,
    padding: Padding,
    spacing: f32,
    width: Length,
    height: Length,
}

impl<'a, Message> Tabs<'a, Message>
where
    Message: 'a,
{
    /// Sets the per-tab padding around each child's content.
    pub fn padding(mut self, padding: impl Into<Padding>) -> Self {
        self.padding = padding.into();
        self
    }

    /// Sets the gap between adjacent tabs. The underline cannot bridge
    /// the gap, it slides over it.
    pub fn spacing(mut self, spacing: f32) -> Self {
        self.spacing = spacing;
        self
    }

    /// Sets the width of the tab strip's bounding box.
    pub fn width(mut self, width: impl Into<Length>) -> Self {
        self.width = width.into();
        self
    }

    /// Sets the height of the tab strip's bounding box.
    pub fn height(mut self, height: impl Into<Length>) -> Self {
        self.height = height.into();
        self
    }
}

impl<'a, Message> From<Tabs<'a, Message>> for Element<'a, Message>
where
    Message: 'a,
{
    fn from(tabs: Tabs<'a, Message>) -> Self {
        Element::new(tabs)
    }
}

#[derive(Clone, Copy, Default)]
struct TabSlot {
    press: PressState,
    last_layout: Rectangle,
}

#[derive(Clone, Copy)]
struct UnderlineTrack {
    /// Eased source rectangle for the current slide. `None` on the first
    /// frame and immediately after construction, so the underline snaps
    /// to the target without an opening animation.
    from: Option<Rectangle>,
    target_index: usize,
    started: Instant,
}

struct TabsState {
    tabs: Vec<TabSlot>,
    underline: UnderlineTrack,
    last_selected: usize,
}

/// Linearly eases between two rectangles. Only `x` and `width` move,
/// matching the underline's horizontal slide. `y`/`height` snap to the
/// target so a resize cannot leave the bar floating mid-row.
fn lerp_bar(from: Rectangle, target: Rectangle, eased: f32) -> Rectangle {
    Rectangle {
        x: from.x + (target.x - from.x) * eased,
        y: target.y,
        width: from.width + (target.width - from.width) * eased,
        height: target.height,
    }
}

fn underline_eased(started: Instant, now: Instant) -> f32 {
    let raw =
        (now.duration_since(started).as_secs_f32() / FADE.as_secs_f32()).clamp(0.0, 1.0);
    easing::EMPHASIZED_DECELERATE.y_at_x(raw)
}

fn current_underline(state: &TabsState, now: Instant) -> Rectangle {
    let target = state
        .tabs
        .get(state.underline.target_index)
        .map(|slot| slot.last_layout)
        .unwrap_or_default();

    match state.underline.from {
        None => target,
        Some(from) => {
            lerp_bar(from, target, underline_eased(state.underline.started, now))
        },
    }
}

impl<'a, Message> Widget<Message, iced::Theme, iced::Renderer> for Tabs<'a, Message>
where
    Message: 'a,
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
        let inner_limits = layout::Limits::NONE;
        let pad_h = self.padding.left + self.padding.right;
        let pad_v = self.padding.top + self.padding.bottom;

        // Pass 1. Measure each child unconstrained so every tab keeps its
        // intrinsic width regardless of the parent's allocation.
        let mut measured = Vec::with_capacity(self.tabs.len());
        let mut row_height: f32 = 0.0;
        for (tab, tab_tree) in self.tabs.iter_mut().zip(tree.children.iter_mut()) {
            let inner = tab
                .as_widget_mut()
                .layout(tab_tree, renderer, &inner_limits);
            let inner_size = inner.size();

            row_height = row_height.max(inner_size.height + pad_v);
            measured.push((inner, inner_size));
        }

        // Pass 2. Bottom-align every tab so all underlines sit on the
        // same baseline regardless of per-tab content height.
        let mut nodes = Vec::with_capacity(measured.len());
        let mut cursor_x: f32 = 0.0;
        for (inner, inner_size) in measured {
            let outer_w = inner_size.width + pad_h;
            let outer_h = inner_size.height + pad_v;
            let outer_y = row_height - outer_h;

            let inner_positioned =
                inner.move_to(Point::new(self.padding.left, self.padding.top));
            let outer = layout::Node::with_children(
                Size::new(outer_w, outer_h),
                vec![inner_positioned],
            )
            .move_to(Point::new(cursor_x, outer_y));

            nodes.push(outer);
            cursor_x += outer_w + self.spacing;
        }
        if !nodes.is_empty() {
            cursor_x -= self.spacing;
        }

        let state = tree.state.downcast_mut::<TabsState>();
        for (slot, node) in state.tabs.iter_mut().zip(nodes.iter()) {
            slot.last_layout = node.bounds();
        }

        let intrinsic = Size::new(cursor_x, row_height + UNDERLINE_THICKNESS);
        let total = limits.resolve(self.width, self.height, intrinsic);

        layout::Node::with_children(total, nodes)
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
        let state = tree.state.downcast_ref::<TabsState>();
        let now = Instant::now();

        for (i, ((tab, tab_tree), tab_layout)) in self
            .tabs
            .iter()
            .zip(tree.children.iter())
            .zip(layout.children())
            .enumerate()
        {
            let slot = &state.tabs[i];
            let resting = if i == self.selected {
                color::TEXT_PRIMARY
            } else {
                color::TEXT_SECONDARY
            };
            let factor = if i == self.selected {
                0.0
            } else {
                slot.press.hover.current(now)
            };

            let cascade = RendererStyle {
                text_color: color::ease(resting, color::TEXT_PRIMARY, factor),
            };
            let content_layout = tab_layout.children().next().expect("tab inner layout");

            tab.as_widget().draw(
                tab_tree,
                renderer,
                theme,
                &cascade,
                content_layout,
                cursor,
                viewport,
            );
        }

        if state.tabs.is_empty() {
            return;
        }

        let bar = current_underline(state, now);
        if bar.width <= 0.0 {
            return;
        }

        // Cached per-tab bounds are widget-local. `fill_quad` paints in
        // window space, so offset by the strip's own absolute position.
        let strip = layout.position();
        let underline_rect = Rectangle {
            x: strip.x + bar.x,
            y: strip.y + bar.y + bar.height,
            width: bar.width,
            height: UNDERLINE_THICKNESS,
        };

        renderer.fill_quad(
            Quad {
                bounds: underline_rect,
                border: Border {
                    radius: UNDERLINE_RADIUS.into(),
                    ..Border::default()
                },
                ..Quad::default()
            },
            color::primary(),
        );
    }

    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<TabsState>()
    }

    fn state(&self) -> tree::State {
        let target = self.selected.min(self.tabs.len().saturating_sub(1));

        tree::State::new(TabsState {
            tabs: vec![TabSlot::default(); self.tabs.len()],
            underline: UnderlineTrack {
                from: None,
                target_index: target,
                started: Instant::now() - FADE,
            },
            last_selected: target,
        })
    }

    fn children(&self) -> Vec<Tree> {
        self.tabs.iter().map(Tree::new).collect()
    }

    fn diff(&self, tree: &mut Tree) {
        tree.diff_children(&self.tabs);

        let state = tree.state.downcast_mut::<TabsState>();
        if state.tabs.len() != self.tabs.len() {
            state.tabs.resize(self.tabs.len(), TabSlot::default());
        }
        if state.tabs.is_empty() {
            return;
        }

        let target = self.selected.min(state.tabs.len() - 1);
        if target == state.last_selected {
            return;
        }

        // Capture the underline's live rectangle as the new slide's
        // source so a mid-flight reversal continues from where it is
        // rather than snapping back to the prior target.
        let now = Instant::now();
        let from = current_underline(state, now);

        state.underline.from = Some(from);
        state.underline.target_index = target;
        state.underline.started = now;
        state.last_selected = target;
    }

    fn operate(
        &mut self,
        tree: &mut Tree,
        layout: Layout<'_>,
        renderer: &iced::Renderer,
        operation: &mut dyn Operation,
    ) {
        for ((tab, tab_tree), tab_layout) in self
            .tabs
            .iter_mut()
            .zip(tree.children.iter_mut())
            .zip(layout.children())
        {
            let content_layout = tab_layout.children().next().expect("tab inner layout");
            tab.as_widget_mut()
                .operate(tab_tree, content_layout, renderer, operation);
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
        // Forward to children first so any nested interactive widget can
        // claim capture before the tab strip dispatches.
        for ((tab, tab_tree), tab_layout) in self
            .tabs
            .iter_mut()
            .zip(tree.children.iter_mut())
            .zip(layout.children())
        {
            let content_layout = tab_layout.children().next().expect("tab inner layout");
            tab.as_widget_mut().update(
                tab_tree,
                event,
                content_layout,
                cursor,
                renderer,
                clipboard,
                shell,
                viewport,
            );
        }

        let now = Instant::now();
        let bounds: Vec<Rectangle> = layout.children().map(|l| l.bounds()).collect();
        let state = tree.state.downcast_mut::<TabsState>();

        match event {
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) => {
                if shell.is_event_captured() {
                    return;
                }

                for (i, slot) in state.tabs.iter_mut().enumerate() {
                    if i == self.selected {
                        continue;
                    }

                    let over =
                        bounds.get(i).map(|b| cursor.is_over(*b)).unwrap_or(false);
                    if slot.press.press(over, now) {
                        shell.request_redraw();
                    }
                }
            },

            Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)) => {
                let mut dispatch_index: Option<usize> = None;

                for (i, slot) in state.tabs.iter_mut().enumerate() {
                    if i == self.selected {
                        continue;
                    }

                    let over =
                        bounds.get(i).map(|b| cursor.is_over(*b)).unwrap_or(false);
                    let was_pressed = slot.press.pressed;
                    let dispatch = slot.press.release(over, now);

                    if was_pressed {
                        shell.request_redraw();
                    }
                    if dispatch && dispatch_index.is_none() {
                        dispatch_index = Some(i);
                    }
                }

                if let Some(i) = dispatch_index
                    && !shell.is_event_captured()
                {
                    shell.publish((self.on_select)(i));
                    shell.capture_event();
                }
            },

            _ => {
                let mut any_redraw = false;
                for (i, slot) in state.tabs.iter_mut().enumerate() {
                    let over = if i == self.selected {
                        false
                    } else {
                        bounds.get(i).map(|b| cursor.is_over(*b)).unwrap_or(false)
                    };
                    if slot.press.reconcile(over, now) {
                        any_redraw = true;
                    }
                }
                if any_redraw {
                    shell.request_redraw();
                }

                if let Event::Window(window::Event::RedrawRequested(_)) = event {
                    let slot_animating =
                        state.tabs.iter().any(|s| s.press.animating(now));
                    let underline_animating = state.underline.from.is_some()
                        && now.duration_since(state.underline.started) < FADE;

                    if slot_animating || underline_animating {
                        shell.request_redraw();
                    }
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
        for ((tab, tab_tree), tab_layout) in self
            .tabs
            .iter()
            .zip(tree.children.iter())
            .zip(layout.children())
        {
            let content_layout = tab_layout.children().next().expect("tab inner layout");
            let inner = tab.as_widget().mouse_interaction(
                tab_tree,
                content_layout,
                cursor,
                viewport,
                renderer,
            );
            if !matches!(inner, mouse::Interaction::None | mouse::Interaction::Idle) {
                return inner;
            }
        }

        for (i, tab_layout) in layout.children().enumerate() {
            if i != self.selected && cursor.is_over(tab_layout.bounds()) {
                return mouse::Interaction::Pointer;
            }
        }

        mouse::Interaction::None
    }
}
