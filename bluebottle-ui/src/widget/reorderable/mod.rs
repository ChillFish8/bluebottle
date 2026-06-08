//! Vertical drag-and-drop reorderable list. Lays N children out in a column.
//! Each row is reordered by pressing the [`grab_handle`] it carries and
//! dragging. Neighbours animate into their new slots over 100 ms.

mod handle;

use std::time::{Duration, Instant};

pub use handle::grab_handle;
use iced::advanced::renderer::Style;
use iced::advanced::widget::{Operation, Tree, tree};
use iced::advanced::{Clipboard, Layout, Renderer, Shell, Widget, layout, overlay};
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

use crate::animate::hover::EPSILON;
use crate::{easing, spacing};

const SLOT_FADE: Duration = Duration::from_millis(100);
const DRAG_THRESHOLD: f32 = 4.0;

/// Builds a reorderable list around `children`. Pressing a child's
/// [`grab_handle`] starts a drag. On release, when the drop slot differs from
/// the press slot, `on_reorder(from, to)` fires.
pub fn reorderable<'a, Message>(
    children: Vec<Element<'a, Message>>,
    on_reorder: impl Fn(usize, usize) -> Message + 'a,
) -> Reorderable<'a, Message>
where
    Message: 'a,
{
    Reorderable {
        children,
        on_reorder: Box::new(on_reorder),
        spacing: spacing::GAP_12,
        width: Length::Fill,
    }
}

/// A vertical reorderable list built by [`reorderable`].
pub struct Reorderable<'a, Message> {
    children: Vec<Element<'a, Message>>,
    on_reorder: Box<dyn Fn(usize, usize) -> Message + 'a>,
    spacing: f32,
    width: Length,
}

impl<'a, Message> Reorderable<'a, Message>
where
    Message: 'a,
{
    /// Vertical gap between rows. Defaults to 12 px.
    pub fn spacing(mut self, gap: f32) -> Self {
        self.spacing = gap;
        self
    }

    /// Width of the list. Defaults to [`Length::Fill`].
    pub fn width(mut self, width: impl Into<Length>) -> Self {
        self.width = width.into();
        self
    }
}

/// A signed eased pixel offset. A pixel-valued cousin of
/// [`crate::animate::hover::Hover`].
#[derive(Clone, Copy)]
struct SlotTween {
    from: f32,
    target: f32,
    started: Instant,
    fade: Duration,
}

impl SlotTween {
    fn settled() -> Self {
        Self {
            from: 0.0,
            target: 0.0,
            started: Instant::now() - SLOT_FADE,
            fade: SLOT_FADE,
        }
    }

    fn current(&self, now: Instant) -> f32 {
        let raw = (now.duration_since(self.started).as_secs_f32()
            / self.fade.as_secs_f32())
        .clamp(0.0, 1.0);
        let curve = if self.target >= self.from {
            &easing::EMPHASIZED_DECELERATE
        } else {
            &easing::EMPHASIZED_ACCELERATE
        };
        let eased = curve.y_at_x(raw);
        self.from + (self.target - self.from) * eased
    }

    fn retarget(&mut self, target: f32, now: Instant) -> bool {
        if (target - self.target).abs() < EPSILON {
            return false;
        }
        self.from = self.current(now);
        self.target = target;
        self.started = now;
        true
    }

    fn animating(&self, now: Instant) -> bool {
        now.duration_since(self.started) < self.fade
    }
}

struct DragState {
    /// Original index of the row being dragged.
    source: usize,
    /// Press point in widget-local coordinates.
    press_y: f32,
    /// Cursor offset from the source row's top at press time.
    grab_offset_y: f32,
    /// Current cursor Y in widget-local coordinates.
    cursor_y: f32,
    /// True once movement exceeded [`DRAG_THRESHOLD`].
    active: bool,
    /// Live insertion index, recomputed on every cursor move.
    target: usize,
}

#[derive(Default)]
struct ReorderState {
    slots: Vec<SlotTween>,
    drag: Option<DragState>,
}

impl ReorderState {
    fn ensure_slots(&mut self, n: usize) {
        if self.slots.len() != n {
            self.slots = (0..n).map(|_| SlotTween::settled()).collect();
        }
    }

    fn active_source(&self) -> Option<usize> {
        self.drag.as_ref().filter(|d| d.active).map(|d| d.source)
    }
}

impl<'a, Message> From<Reorderable<'a, Message>> for Element<'a, Message>
where
    Message: 'a,
{
    fn from(value: Reorderable<'a, Message>) -> Self {
        Element::new(value)
    }
}

impl<'a, Message> Widget<Message, iced::Theme, iced::Renderer>
    for Reorderable<'a, Message>
where
    Message: 'a,
{
    fn size(&self) -> Size<Length> {
        Size {
            width: self.width,
            height: Length::Shrink,
        }
    }

    fn layout(
        &mut self,
        tree: &mut Tree,
        renderer: &iced::Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        let max = limits.max();
        let row_limits =
            layout::Limits::new(Size::ZERO, Size::new(max.width, f32::INFINITY))
                .width(self.width);

        let mut nodes = Vec::with_capacity(self.children.len());
        let mut y = 0.0_f32;
        let mut max_width = 0.0_f32;

        for (i, child) in self.children.iter_mut().enumerate() {
            let node = child.as_widget_mut().layout(
                &mut tree.children[i],
                renderer,
                &row_limits,
            );
            let size = node.size();
            max_width = max_width.max(size.width);

            nodes.push(node.move_to(Point::new(0.0, y)));
            y += size.height + self.spacing;
        }

        let total_height = if self.children.is_empty() {
            0.0
        } else {
            y - self.spacing
        };

        let state = tree.state.downcast_mut::<ReorderState>();
        state.ensure_slots(self.children.len());

        // Resolve the chassis width through `limits` so a `Length::Fill` widget
        // reports the parent-assigned width rather than the widest child.
        let size = limits.resolve(
            self.width,
            Length::Shrink,
            Size::new(max_width, total_height),
        );
        layout::Node::with_children(size, nodes)
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
        let state = tree.state.downcast_ref::<ReorderState>();
        let now = Instant::now();
        let bounds = layout.bounds();

        for (i, (child, child_layout)) in
            self.children.iter().zip(layout.children()).enumerate()
        {
            let row_top_local = child_layout.bounds().y - bounds.y;
            let translation = row_translation(state, i, row_top_local, now);

            let child_cursor = if state.active_source() == Some(i) {
                mouse::Cursor::Unavailable
            } else {
                cursor * Transformation::translate(-translation.x, -translation.y)
            };

            let child_viewport = Rectangle {
                x: viewport.x - translation.x,
                y: viewport.y - translation.y,
                width: viewport.width,
                height: viewport.height,
            };

            renderer.with_translation(translation, |renderer| {
                child.as_widget().draw(
                    &tree.children[i],
                    renderer,
                    theme,
                    style,
                    child_layout,
                    child_cursor,
                    &child_viewport,
                );
            });
        }
    }

    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<ReorderState>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(ReorderState::default())
    }

    fn children(&self) -> Vec<Tree> {
        self.children.iter().map(Tree::new).collect()
    }

    fn diff(&self, tree: &mut Tree) {
        tree.diff_children(&self.children);
        let state = tree.state.downcast_mut::<ReorderState>();
        let count_changed = state.slots.len() != self.children.len();
        state.ensure_slots(self.children.len());
        if count_changed {
            // The row vector grew or shrank under us, so the in-flight indices
            // no longer map to the user's intended rows. Drop the drag rather
            // than reorder against a different list.
            state.drag = None;
        } else if let Some(drag) = &state.drag
            && (drag.source >= self.children.len() || drag.target >= self.children.len())
        {
            state.drag = None;
        }
    }

    fn operate(
        &mut self,
        tree: &mut Tree,
        layout: Layout<'_>,
        renderer: &iced::Renderer,
        operation: &mut dyn Operation,
    ) {
        for (i, (child, child_layout)) in
            self.children.iter_mut().zip(layout.children()).enumerate()
        {
            child.as_widget_mut().operate(
                &mut tree.children[i],
                child_layout,
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
        let drag_active = tree
            .state
            .downcast_ref::<ReorderState>()
            .drag
            .as_ref()
            .is_some_and(|d| d.active);
        let is_mouse_event = matches!(event, Event::Mouse(_));

        // Forward to every child so internal interactive widgets keep working.
        // While a drag is active we suppress mouse events to children so a
        // stray release does not register as a click on whatever row sits
        // under the cursor, and hover state does not flicker against shifted
        // slots whose layout positions diverge from their visuals.
        if !(drag_active && is_mouse_event) {
            for (i, (child, child_layout)) in
                self.children.iter_mut().zip(layout.children()).enumerate()
            {
                child.as_widget_mut().update(
                    &mut tree.children[i],
                    event,
                    child_layout,
                    cursor,
                    renderer,
                    clipboard,
                    shell,
                    viewport,
                );
                if shell.is_event_captured() {
                    return;
                }
            }
        }

        let bounds = layout.bounds();
        let cursor_pos = cursor.position();
        let now = Instant::now();
        let state = tree.state.downcast_mut::<ReorderState>();

        match event {
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) => {
                let Some(pos) = cursor_pos else { return };
                if shell.is_event_captured() {
                    return;
                }
                if !bounds.contains(pos) {
                    return;
                }
                let local_y = pos.y - bounds.y;

                for (i, child_layout) in layout.children().enumerate() {
                    if let Some(rect) = handle::find_in(&tree.children[i], child_layout)
                        && rect.contains(pos)
                    {
                        let row_top = child_layout.bounds().y - bounds.y;
                        state.drag = Some(DragState {
                            source: i,
                            press_y: local_y,
                            grab_offset_y: local_y - row_top,
                            cursor_y: local_y,
                            active: false,
                            target: i,
                        });
                        shell.capture_event();
                        shell.request_redraw();
                        return;
                    }
                }
            },

            Event::Mouse(mouse::Event::CursorMoved { .. }) => {
                let Some(drag) = state.drag.as_mut() else {
                    return;
                };
                let Some(pos) = cursor_pos else { return };
                drag.cursor_y = pos.y - bounds.y;

                if !drag.active && (drag.cursor_y - drag.press_y).abs() > DRAG_THRESHOLD
                {
                    drag.active = true;
                }

                if drag.active {
                    let rows = row_geometries(layout, bounds);
                    drag.target = compute_target(&rows, drag.source, drag.cursor_y);
                    apply_slot_targets(state, layout, bounds, now);
                    shell.request_redraw();
                }
            },

            Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)) => {
                let Some(mut drag) = state.drag.take() else {
                    return;
                };

                // Re-read the cursor at release time so a drag that ended
                // outside the chassis still commits against the final cursor
                // position rather than the last in-bounds CursorMoved.
                if let Some(pos) = cursor_pos {
                    drag.cursor_y = pos.y - bounds.y;
                }

                if drag.active {
                    let rows = row_geometries(layout, bounds);
                    drag.target = compute_target(&rows, drag.source, drag.cursor_y);
                    state.slots = release_slots(&drag, &state.slots, &rows, now);
                    if drag.target != drag.source {
                        shell.publish((self.on_reorder)(drag.source, drag.target));
                    }
                    shell.capture_event();
                }

                shell.request_redraw();
            },

            Event::Window(window::Event::RedrawRequested(_))
                if state.drag.is_some()
                    || state.slots.iter().any(|s| s.animating(now)) =>
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
        let state = tree.state.downcast_ref::<ReorderState>();

        if state.drag.as_ref().is_some_and(|d| d.active) {
            return mouse::Interaction::Grabbing;
        }

        if let Some(pos) = cursor.position() {
            for (i, child_layout) in layout.children().enumerate() {
                if let Some(rect) = handle::find_in(&tree.children[i], child_layout)
                    && rect.contains(pos)
                {
                    return mouse::Interaction::Grab;
                }
            }
        }

        for (i, (child, child_layout)) in
            self.children.iter().zip(layout.children()).enumerate()
        {
            let inner = child.as_widget().mouse_interaction(
                &tree.children[i],
                child_layout,
                cursor,
                viewport,
                renderer,
            );
            if !matches!(inner, mouse::Interaction::None | mouse::Interaction::Idle) {
                return inner;
            }
        }
        mouse::Interaction::None
    }

    fn overlay<'b>(
        &'b mut self,
        tree: &'b mut Tree,
        layout: Layout<'b>,
        renderer: &iced::Renderer,
        viewport: &Rectangle,
        translation: Vector,
    ) -> Option<overlay::Element<'b, Message, iced::Theme, iced::Renderer>> {
        let Tree {
            state: tree_state,
            children: tree_children,
            ..
        } = tree;
        // While the source row is detached from its slot, suppress its overlay
        // so a popover or tooltip anchored to the row's layout position does
        // not visually disconnect from the row.
        let active_source = tree_state.downcast_ref::<ReorderState>().active_source();

        let children = self
            .children
            .iter_mut()
            .zip(tree_children.iter_mut())
            .zip(layout.children())
            .enumerate()
            .filter(|(i, _)| Some(*i) != active_source)
            .filter_map(|(_, ((child, sub_tree), child_layout))| {
                child.as_widget_mut().overlay(
                    sub_tree,
                    child_layout,
                    renderer,
                    viewport,
                    translation,
                )
            })
            .collect::<Vec<_>>();

        (!children.is_empty()).then(|| overlay::Group::with_children(children).overlay())
    }
}

#[derive(Clone, Copy)]
struct RowGeometry {
    top: f32,
    height: f32,
}

fn row_geometries(layout: Layout<'_>, bounds: Rectangle) -> Vec<RowGeometry> {
    layout
        .children()
        .map(|child| {
            let b = child.bounds();
            RowGeometry {
                top: b.y - bounds.y,
                height: b.height,
            }
        })
        .collect()
}

/// Walks the rows starting from `source` and adjusts the target index by
/// every row midpoint the cursor has crossed in either direction.
fn compute_target(rows: &[RowGeometry], source: usize, cursor_y: f32) -> usize {
    let mut target = source;
    while target > 0 {
        let mid = rows[target - 1].top + rows[target - 1].height * 0.5;
        if cursor_y < mid {
            target -= 1;
        } else {
            break;
        }
    }
    while target + 1 < rows.len() {
        let mid = rows[target + 1].top + rows[target + 1].height * 0.5;
        if cursor_y > mid {
            target += 1;
        } else {
            break;
        }
    }
    target
}

/// Updates each slot's eased target so neighbours slide into the space the
/// dragged row would vacate.
fn apply_slot_targets(
    state: &mut ReorderState,
    layout: Layout<'_>,
    bounds: Rectangle,
    now: Instant,
) {
    let Some(drag) = state.drag.as_ref() else {
        return;
    };
    let rows = row_geometries(layout, bounds);
    if rows.is_empty() {
        return;
    }
    let source_shift = rows[drag.source].height + spacing_between(&rows, drag.source);

    for i in 0..rows.len() {
        let desired = if i == drag.source {
            0.0
        } else if drag.target > drag.source && i > drag.source && i <= drag.target {
            -source_shift
        } else if drag.target < drag.source && i >= drag.target && i < drag.source {
            source_shift
        } else {
            0.0
        };
        state.slots[i].retarget(desired, now);
    }
}

/// Gap between the bottom of row `i` and the top of the next row, derived
/// from the layout so the algorithm does not need to know the chassis
/// spacing constant.
fn spacing_between(rows: &[RowGeometry], i: usize) -> f32 {
    if i + 1 < rows.len() {
        rows[i + 1].top - (rows[i].top + rows[i].height)
    } else if i > 0 {
        rows[i].top - (rows[i - 1].top + rows[i - 1].height)
    } else {
        0.0
    }
}

/// Builds the slot vector that should apply to the post-release layout. Each
/// row's slot starts at the offset between its current visual position and the
/// row's new resting position, then eases to zero. Without this, the source
/// would snap from the cursor back to its old slot before the caller's next
/// render moves it to its new slot.
fn release_slots(
    drag: &DragState,
    old_slots: &[SlotTween],
    rows: &[RowGeometry],
    now: Instant,
) -> Vec<SlotTween> {
    let mut next = vec![SlotTween::settled(); rows.len()];

    for old in 0..rows.len() {
        let new_idx = remap_index(old, drag.source, drag.target);

        let visual = if old == drag.source {
            drag.cursor_y - drag.grab_offset_y
        } else {
            rows[old].top + old_slots[old].current(now)
        };
        let natural_new = rows[new_idx].top;
        let from = visual - natural_new;

        next[new_idx] = if from.abs() > EPSILON {
            SlotTween {
                from,
                target: 0.0,
                started: now,
                fade: SLOT_FADE,
            }
        } else {
            SlotTween::settled()
        };
    }

    next
}

/// Maps a row's pre-reorder index to its post-reorder index. Source slides
/// into target. Rows squeezed between the two indices shift one slot in the
/// opposite direction.
fn remap_index(old: usize, source: usize, target: usize) -> usize {
    if target == source {
        old
    } else if old == source {
        target
    } else if target > source && old > source && old <= target {
        old - 1
    } else if target < source && old >= target && old < source {
        old + 1
    } else {
        old
    }
}

fn row_translation(
    state: &ReorderState,
    i: usize,
    row_top_local: f32,
    now: Instant,
) -> Vector {
    if let Some(drag) = &state.drag
        && drag.active
        && drag.source == i
    {
        let desired = drag.cursor_y - drag.grab_offset_y;
        return Vector::new(0.0, desired - row_top_local);
    }
    Vector::new(0.0, state.slots[i].current(now))
}
