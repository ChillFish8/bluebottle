//! A grouped vertical list with animated jump-to-group / centre-on-child.
//!
//! Groups stack tightly inside themselves (header followed by its children)
//! with a configurable gap between adjacent groups. The widget animates
//! programmatic scroll requests, emits the topmost-visible group's index
//! whenever that changes, and keeps the user's view stable when children
//! below or above the viewport hydrate from skeletons into taller content.

use std::time::Instant;

use iced::advanced::widget::{Operation, Tree, tree};
use iced::advanced::{Clipboard, Layout, Renderer, Shell, Widget, layout, renderer};
use iced::{
    Element,
    Event,
    Length,
    Point,
    Rectangle,
    Size,
    Transformation,
    Vector,
    mouse,
    window,
};

use crate::widget::scroll::ScrollEngine;

/// Fraction of the viewport a group's header may sit below the top before
/// the widget treats that group as the shown one.
const SHOWN_LEAD: f32 = 0.35;

/// A header followed by zero or more children, the unit `smart_list` works
/// with. Built by [`smart_group`].
pub struct SmartGroup<'a, Message> {
    header: Element<'a, Message>,
    children: Vec<Element<'a, Message>>,
}

/// Builds a [`SmartGroup`] from a header element and its child rows.
pub fn smart_group<'a, Message>(
    header: impl Into<Element<'a, Message>>,
    children: impl IntoIterator<Item = impl Into<Element<'a, Message>>>,
) -> SmartGroup<'a, Message> {
    SmartGroup {
        header: header.into(),
        children: children.into_iter().map(Into::into).collect(),
    }
}

/// Builds a smart list from a sequence of [`SmartGroup`]s plus a callback
/// fired with the index of the topmost group whenever that group changes
/// (user scroll, programmatic scroll, reflow, or initial mount).
pub fn smart_list<'a, Message>(
    groups: impl IntoIterator<Item = SmartGroup<'a, Message>>,
    on_shown_group: impl Fn(usize) -> Message + 'a,
) -> SmartList<'a, Message> {
    let mut flat: Vec<Element<'a, Message>> = Vec::new();
    let mut spans: Vec<GroupSpan> = Vec::new();

    for group in groups {
        let header_idx = flat.len();
        flat.push(group.header);
        let first_child_idx = flat.len();
        let child_count = group.children.len();
        flat.extend(group.children);

        spans.push(GroupSpan {
            header_idx,
            first_child_idx,
            child_count,
        });
    }

    SmartList {
        flat,
        spans,
        on_shown_group: Box::new(on_shown_group),
        on_target_finished: None,
        width: Length::Fill,
        height: Length::Fill,
        spacing: 16.0,
        show_group: None,
        show_child: None,
    }
}

/// A scrollable list of grouped entries.
pub struct SmartList<'a, Message> {
    flat: Vec<Element<'a, Message>>,
    spans: Vec<GroupSpan>,
    on_shown_group: Box<dyn Fn(usize) -> Message + 'a>,
    on_target_finished: Option<Box<dyn Fn() -> Message + 'a>>,
    width: Length,
    height: Length,
    spacing: f32,
    show_group: Option<usize>,
    show_child: Option<usize>,
}

impl<'a, Message> SmartList<'a, Message> {
    pub fn width(mut self, width: impl Into<Length>) -> Self {
        self.width = width.into();
        self
    }

    pub fn height(mut self, height: impl Into<Length>) -> Self {
        self.height = height.into();
        self
    }

    /// Gap between adjacent groups. Headers and children inside a group are
    /// stacked tightly without extra spacing.
    pub fn spacing(mut self, spacing: f32) -> Self {
        self.spacing = spacing;
        self
    }

    /// Animates an offset that brings the given group's header to the top
    /// of the viewport. `None` cancels any active animation.
    pub fn show_group(mut self, index: Option<usize>) -> Self {
        self.show_group = index;
        self
    }

    /// Pairs with [`Self::show_group`] to centre a specific child of that
    /// group in the viewport instead of aligning the header. Ignored when
    /// `show_group` is `None`.
    pub fn show_child(mut self, index: Option<usize>) -> Self {
        self.show_child = index;
        self
    }

    /// Called once a programmatic scroll target finishes, whether by
    /// completing the animation or by being cancelled because the user
    /// grabbed the scrollbar. Hosts typically reset their stored
    /// `show_group` / `show_child` in response, so the next click on the
    /// same button triggers a fresh jump.
    pub fn on_target_finished(mut self, on_finished: impl Fn() -> Message + 'a) -> Self {
        self.on_target_finished = Some(Box::new(on_finished));
        self
    }
}

impl<'a, Message> From<SmartList<'a, Message>> for Element<'a, Message>
where
    Message: 'a,
{
    fn from(list: SmartList<'a, Message>) -> Self {
        Element::new(list)
    }
}

#[derive(Clone, Copy)]
struct GroupSpan {
    header_idx: usize,
    first_child_idx: usize,
    child_count: usize,
}

/// A stable identifier for an item in the list plus its current y. Tracked
/// across frames so the offset can follow content that shifts under
/// hydration.
#[derive(Clone, Copy)]
struct Anchor {
    key: u64,
    y: f32,
}

#[derive(Clone, Copy, Default)]
struct State {
    engine: ScrollEngine,
    last_request: (Option<usize>, Option<usize>),
    last_shown: Option<usize>,
    anchor: Option<Anchor>,
    /// Cached at the end of layout so update and draw avoid re-walking the
    /// child layouts to find the bottom.
    content_height: f32,
}

impl<'a, Message> Widget<Message, iced::Theme, iced::Renderer>
    for SmartList<'a, Message>
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
        let limits = limits.width(self.width).height(self.height);
        let viewport = limits.max();
        let inner =
            layout::Limits::new(Size::ZERO, Size::new(viewport.width, f32::INFINITY));

        let mut nodes: Vec<layout::Node> = Vec::with_capacity(self.flat.len());
        let mut y = 0.0_f32;

        for gi in 0..self.spans.len() {
            let span = self.spans[gi];

            if gi > 0 {
                y += self.spacing;
            }

            let header_node = self.flat[span.header_idx]
                .as_widget_mut()
                .layout(&mut tree.children[span.header_idx], renderer, &inner)
                .move_to(Point::new(0.0, y));
            y += header_node.bounds().height;
            nodes.push(header_node);

            for ci in 0..span.child_count {
                let idx = span.first_child_idx + ci;
                let node = self.flat[idx]
                    .as_widget_mut()
                    .layout(&mut tree.children[idx], renderer, &inner)
                    .move_to(Point::new(0.0, y));
                y += node.bounds().height;
                nodes.push(node);
            }
        }

        let state = tree.state.downcast_mut::<State>();
        state.content_height = y;

        layout::Node::with_children(viewport, nodes)
    }

    fn draw(
        &self,
        tree: &Tree,
        renderer: &mut iced::Renderer,
        theme: &iced::Theme,
        style: &renderer::Style,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        _viewport: &Rectangle,
    ) {
        let bounds = layout.bounds();
        let state = tree.state.downcast_ref::<State>();
        let offset = state.engine.offset;
        let shift = Transformation::translate(0.0, offset);

        renderer.with_layer(bounds, |renderer| {
            renderer.with_translation(Vector::new(0.0, -offset), |renderer| {
                for ((elem, child_tree), child_layout) in self
                    .flat
                    .iter()
                    .zip(tree.children.iter())
                    .zip(layout.children())
                {
                    elem.as_widget().draw(
                        child_tree,
                        renderer,
                        theme,
                        style,
                        child_layout,
                        cursor * shift,
                        &(bounds * shift),
                    );
                }
            });
        });

        state
            .engine
            .bar
            .draw(renderer, bounds, state.content_height, offset);
    }

    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<State>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(State::default())
    }

    fn children(&self) -> Vec<Tree> {
        self.flat.iter().map(Tree::new).collect()
    }

    fn diff(&self, tree: &mut Tree) {
        tree.diff_children(&self.flat);

        let state = tree.state.downcast_mut::<State>();
        let req = (self.show_group, self.show_child);
        if state.last_request != req {
            state.last_request = req;
            if self.show_group.is_some() {
                state.engine.start_target(Instant::now());
            } else {
                state.engine.clear_target();
            }
        }
    }

    fn operate(
        &mut self,
        tree: &mut Tree,
        layout: Layout<'_>,
        renderer: &iced::Renderer,
        operation: &mut dyn Operation,
    ) {
        for ((elem, child_tree), child_layout) in self
            .flat
            .iter_mut()
            .zip(tree.children.iter_mut())
            .zip(layout.children())
        {
            elem.as_widget_mut()
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
        let bounds = layout.bounds();
        let now = Instant::now();
        let state = tree.state.downcast_mut::<State>();
        let content_height = state.content_height;
        let max_offset = (content_height - bounds.height).max(0.0);

        let bar_event = state.engine.bar.update(
            event,
            bounds,
            content_height,
            state.engine.offset,
            cursor,
            shell,
        );
        let captured = bar_event.captured();
        let is_redraw =
            matches!(event, Event::Window(window::Event::RedrawRequested(_)));

        // Skip metrics work on bystander events (most CursorMoveds, keyboard,
        // etc.) and just keep the offset within the current scroll range.
        if !captured && !is_redraw {
            state.engine.clamp(max_offset);

            let shift = Transformation::translate(0.0, state.engine.offset);

            for ((elem, child_tree), child_layout) in self
                .flat
                .iter_mut()
                .zip(tree.children.iter_mut())
                .zip(layout.children())
            {
                elem.as_widget_mut().update(
                    child_tree,
                    event,
                    child_layout,
                    cursor * shift,
                    renderer,
                    clipboard,
                    shell,
                    &(*viewport * shift),
                );
            }
            return;
        }

        let metrics = Metrics::collect(layout, &self.spans);

        // Anchor shift uses the pre-clamp offset so a shrink that pushes the
        // previous offset past the new max still tracks the anchored item.
        if let Some(prev) = state.anchor
            && let Some(now_y) = metrics.lookup(prev.key)
        {
            state.engine.shift_offset(now_y - prev.y, max_offset);
        }
        state.engine.clamp(max_offset);

        let had_target = state.engine.has_target();
        state.engine.apply_bar_event(bar_event, max_offset, now);

        if is_redraw {
            match metrics.resolve_target(bounds.height, self.show_group, self.show_child)
            {
                Some(to) => {
                    state.engine.step_target(now, to, max_offset);
                },
                None if state.engine.target_expired(now) => {
                    state.engine.clear_target();
                },
                None => {},
            }
        }

        if had_target
            && !state.engine.has_target()
            && let Some(cb) = &self.on_target_finished
        {
            shell.publish(cb());
        }

        let shown = metrics.shown_group(state.engine.offset, bounds.height);
        if state.last_shown != Some(shown) {
            state.last_shown = Some(shown);
            shell.publish((self.on_shown_group)(shown));
        }

        state.anchor = metrics.anchor_at(state.engine.offset);

        if is_redraw && state.engine.animating(now) {
            shell.request_redraw();
        }

        if captured {
            return;
        }

        let shift = Transformation::translate(0.0, state.engine.offset);

        for ((elem, child_tree), child_layout) in self
            .flat
            .iter_mut()
            .zip(tree.children.iter_mut())
            .zip(layout.children())
        {
            elem.as_widget_mut().update(
                child_tree,
                event,
                child_layout,
                cursor * shift,
                renderer,
                clipboard,
                shell,
                &(*viewport * shift),
            );
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
        let bounds = layout.bounds();
        let state = tree.state.downcast_ref::<State>();
        let offset = state.engine.offset;

        if let Some(interaction) = state.engine.bar.mouse_interaction(
            bounds,
            state.content_height,
            offset,
            cursor,
        ) {
            return interaction;
        }

        let shift = Transformation::translate(0.0, offset);

        self.flat
            .iter()
            .zip(tree.children.iter())
            .zip(layout.children())
            .map(|((elem, child_tree), child_layout)| {
                elem.as_widget().mouse_interaction(
                    child_tree,
                    child_layout,
                    cursor * shift,
                    &(*viewport * shift),
                    renderer,
                )
            })
            .max()
            .unwrap_or_default()
    }
}

/// Per-frame layout introspection used to resolve targets, anchors, and
/// the currently-shown group.
struct Metrics {
    groups: Vec<GroupMetrics>,
}

struct GroupMetrics {
    header_y: f32,
    children: Vec<ChildMetrics>,
}

struct ChildMetrics {
    y: f32,
    height: f32,
}

impl Metrics {
    fn collect(layout: Layout<'_>, spans: &[GroupSpan]) -> Self {
        let origin_y = layout.bounds().y;
        let nodes: Vec<Layout<'_>> = layout.children().collect();

        let groups = spans
            .iter()
            .map(|span| {
                let header_y = nodes[span.header_idx].bounds().y - origin_y;
                let children = (0..span.child_count)
                    .map(|ci| {
                        let b = nodes[span.first_child_idx + ci].bounds();
                        ChildMetrics {
                            y: b.y - origin_y,
                            height: b.height,
                        }
                    })
                    .collect();

                GroupMetrics { header_y, children }
            })
            .collect();

        Self { groups }
    }

    /// Largest group index whose header has entered the top 35% of the
    /// viewport. The lead means the host's indicator flips to a group once
    /// its header is visibly approaching the top, rather than waiting for
    /// it to scroll off the edge.
    fn shown_group(&self, offset: f32, viewport_height: f32) -> usize {
        self.group_above(offset + viewport_height * SHOWN_LEAD)
    }

    /// Largest group index whose header sits at or above `y`. Used by both
    /// the anchor (strict, to track the actual topmost item) and the shown
    /// indicator (with a 25% lead, via [`Self::shown_group`]).
    fn group_above(&self, y: f32) -> usize {
        let mut idx = 0;
        for (i, group) in self.groups.iter().enumerate() {
            if group.header_y <= y {
                idx = i;
            } else {
                break;
            }
        }
        idx
    }

    /// Anchor on the child that straddles `offset`, or otherwise on the
    /// header of the topmost group. None for an empty list.
    fn anchor_at(&self, offset: f32) -> Option<Anchor> {
        if self.groups.is_empty() {
            return None;
        }
        let g = self.group_above(offset);
        let group = &self.groups[g];
        for (ci, child) in group.children.iter().enumerate() {
            if child.y <= offset && child.y + child.height > offset {
                return Some(Anchor {
                    key: anchor_key(g, Some(ci)),
                    y: child.y,
                });
            }
        }
        Some(Anchor {
            key: anchor_key(g, None),
            y: group.header_y,
        })
    }

    /// Look up the current y of the item identified by an anchor key.
    fn lookup(&self, key: u64) -> Option<f32> {
        let (g, c) = decode_key(key);
        let group = self.groups.get(g)?;
        match c {
            Some(ci) => group.children.get(ci).map(|child| child.y),
            None => Some(group.header_y),
        }
    }

    /// Resolve the parent's show_group / show_child into a target offset.
    fn resolve_target(
        &self,
        viewport_height: f32,
        show_group: Option<usize>,
        show_child: Option<usize>,
    ) -> Option<f32> {
        let g = show_group?;
        let group = self.groups.get(g)?;
        match show_child {
            Some(ci) => {
                let child = group.children.get(ci)?;
                Some(child.y - (viewport_height - child.height) / 2.0)
            },
            None => Some(group.header_y),
        }
    }
}

fn anchor_key(group: usize, child: Option<usize>) -> u64 {
    let g = group as u64;
    let c = child.map_or(0, |c| c as u64 + 1);
    (g << 32) | c
}

fn decode_key(key: u64) -> (usize, Option<usize>) {
    let g = (key >> 32) as usize;
    let c_raw = (key & 0xFFFF_FFFF) as usize;
    let c = if c_raw == 0 { None } else { Some(c_raw - 1) };
    (g, c)
}
