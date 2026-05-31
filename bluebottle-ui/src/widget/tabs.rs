//! A horizontal tab strip with per-tab centre-anchored underlines.
//!
//! Each tab pairs a Material Icons glyph with a label. The widget owns
//! both so it can fade the icon and the label on independent colour
//! tracks. Hover on an inactive tab grows a half-width half-alpha accent
//! bar out of the tab's centre and tones the label up to [`color::TEXT_HOVER`].
//! Clicking that tab eases the bar into a full-width full-alpha underline
//! and the label to [`color::TEXT_PRIMARY`]. The previously active tab's
//! bar shrinks back into its own centre over the same window.
//!
//! Selected tabs are inert. No pointer cursor, no hover affordance, no
//! press dispatch. Clicking the active tab is a no-op.

use std::borrow::Cow;
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

use crate::animate::hover::{EPSILON, Hover, PressState};
use crate::util::lerp;
use crate::{color, font, icon, text};
use crate::text::Variant;

const UNDERLINE_THICKNESS: f32 = 2.0;
const UNDERLINE_RADIUS: f32 = UNDERLINE_THICKNESS / 2.0;
const ICON_SIZE: f32 = 13.0;
const ICON_LABEL_GAP: f32 = 6.0;
const HOVER_BAR_FRACTION: f32 = 0.5;
const HOVER_BAR_ALPHA: f32 = 0.5;
const DEFAULT_PADDING: Padding = Padding {
    top: 12.0,
    right: 14.0,
    bottom: 12.0,
    left: 14.0,
};
const DEFAULT_SPACING: f32 = 2.0;

/// Creates a tab strip over `items` with `selected` highlighted.
/// `on_select` maps the clicked tab's index to a message. Clicking the
/// already-selected tab publishes nothing.
pub fn tabs<'a, Message>(
    items: impl IntoIterator<Item = Tab>,
    selected: usize,
    on_select: impl Fn(usize) -> Message + 'a,
) -> TabBar<'a, Message>
where
    Message: 'a,
{
    let items: Vec<TabItem> = items.into_iter().map(materialize).collect();

    TabBar {
        items,
        selected,
        on_select: Box::new(on_select),
        padding: DEFAULT_PADDING,
        spacing: DEFAULT_SPACING,
        width: Length::Shrink,
        height: Length::Shrink,
    }
}

/// Builds one [`Tab`] from a Material Icons glyph and a label. The icon
/// is required so every tab carries the same visual rhythm of icon, gap,
/// label.
pub fn tab(icon: &'static str, label: impl Into<Cow<'static, str>>) -> Tab {
    Tab {
        icon,
        label: label.into(),
    }
}

/// One tab in a [`TabBar`]. Built with [`tab`].
#[derive(Clone)]
pub struct Tab {
    icon: &'static str,
    label: Cow<'static, str>,
}

/// A configurable tab strip, built by [`tabs`].
pub struct TabBar<'a, Message> {
    items: Vec<TabItem>,
    selected: usize,
    on_select: Box<dyn Fn(usize) -> Message + 'a>,
    padding: Padding,
    spacing: f32,
    width: Length,
    height: Length,
}

struct TabItem {
    icon: text::Text<'static>,
    label: text::Text<'static>,
}

fn materialize(item: Tab) -> TabItem {
    // The typography role sets an explicit colour. Strip it so the label
    // rides the per-frame cascade this widget hands the child in draw.
    let label = text::label(item.label, Variant::Alt)
        .font(font::medium())
        .inherit_color();

    let icon = icon::filled(item.icon).size(ICON_SIZE);

    TabItem { icon, label }
}

impl<'a, Message> TabBar<'a, Message>
where
    Message: 'a,
{
    /// Sets the per-tab padding around each child's content.
    pub fn padding(mut self, padding: impl Into<Padding>) -> Self {
        self.padding = padding.into();
        self
    }

    /// Sets the gap between adjacent tabs.
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

impl<'a, Message> From<TabBar<'a, Message>> for Element<'a, Message>
where
    Message: 'a,
{
    fn from(bar: TabBar<'a, Message>) -> Self {
        Element::new(bar)
    }
}

#[derive(Clone, Copy)]
struct TabSlot {
    press: PressState,
    active: Hover,
    last_outer: Rectangle,
}

impl TabSlot {
    fn settled(active: bool) -> Self {
        Self {
            press: PressState::default(),
            active: Hover::settled(active),
            last_outer: Rectangle::default(),
        }
    }
}

struct TabsState {
    tabs: Vec<TabSlot>,
    last_selected: usize,
}

impl<'a, Message> TabBar<'a, Message> {
    fn flat_children(
        &self,
    ) -> impl Iterator<Item = &dyn Widget<Message, iced::Theme, iced::Renderer>> {
        self.items
            .iter()
            .flat_map(|item| [item.icon.as_widget(), item.label.as_widget()])
    }
}

impl<'a, Message> Widget<Message, iced::Theme, iced::Renderer> for TabBar<'a, Message>
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

        // Pass 1. Measure each tab's icon and label unconstrained so every
        // tab keeps its intrinsic content size regardless of the parent
        // allocation.
        let mut measured: Vec<TabMeasure> = Vec::with_capacity(self.items.len());
        let mut row_height: f32 = 0.0;
        let mut max_inner_w: f32 = 0.0;
        let mut child_trees = tree.children.iter_mut();

        for item in self.items.iter_mut() {
            let icon_tree = child_trees.next().expect("icon tree");
            let icon_node =
                item.icon
                    .as_widget_mut::<Message>()
                    .layout(icon_tree, renderer, &inner_limits);
            let icon_size = icon_node.size();

            let label_tree = child_trees.next().expect("label tree");
            let label_node =
                item.label
                    .as_widget_mut::<Message>()
                    .layout(label_tree, renderer, &inner_limits);
            let label_size = label_node.size();

            let inner_w = icon_size.width + ICON_LABEL_GAP + label_size.width;
            let inner_h = icon_size.height.max(label_size.height);

            row_height = row_height.max(inner_h + pad_v);
            max_inner_w = max_inner_w.max(inner_w);
            measured.push(TabMeasure {
                icon: (icon_node, icon_size),
                label: (label_node, label_size),
                inner_w,
                inner_h,
            });
        }

        // Every tab takes the widest tab's outer width so a weight or label
        // swap between active and inactive states cannot shift the strip.
        let cell_outer_w = max_inner_w + pad_h;

        // Pass 2. Bottom-align so every tab's underline sits on the same
        // baseline regardless of per-tab content height.
        let mut nodes: Vec<layout::Node> = Vec::with_capacity(measured.len());
        let mut cursor_x: f32 = 0.0;

        for m in measured {
            let outer_h = m.inner_h + pad_v;
            let outer_y = row_height - outer_h;

            // Centre the content horizontally inside the shared cell width
            // so a narrower tab does not anchor its icon to the left edge.
            let content_x = (cell_outer_w - m.inner_w) / 2.0;
            let inner_y = self.padding.top;

            let icon_y = inner_y + (m.inner_h - m.icon.1.height) / 2.0;
            let icon_positioned = m.icon.0.move_to(Point::new(content_x, icon_y));

            let label_x = content_x + m.icon.1.width + ICON_LABEL_GAP;
            let label_y = inner_y + (m.inner_h - m.label.1.height) / 2.0;
            let label_positioned = m.label.0.move_to(Point::new(label_x, label_y));

            let outer = layout::Node::with_children(
                Size::new(cell_outer_w, outer_h),
                vec![icon_positioned, label_positioned],
            )
            .move_to(Point::new(cursor_x, outer_y));

            nodes.push(outer);
            cursor_x += cell_outer_w + self.spacing;
        }
        if !nodes.is_empty() {
            cursor_x -= self.spacing;
        }

        let state = tree.state.downcast_mut::<TabsState>();
        for (slot, node) in state.tabs.iter_mut().zip(nodes.iter()) {
            slot.last_outer = node.bounds();
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
        let strip = layout.position();
        let accent = color::primary();

        let mut child_trees = tree.children.iter();

        for (i, (item, tab_layout)) in
            self.items.iter().zip(layout.children()).enumerate()
        {
            let slot = state
                .tabs
                .get(i)
                .copied()
                .unwrap_or(TabSlot::settled(false));
            // Read the live hover even for the active tab. The press state
            // is already forced toward zero once a tab becomes selected, so
            // it unwinds smoothly alongside the active fade-in. Clipping it
            // to zero here would make the label dip from HOVER to SECONDARY
            // before the active track had moved, which reads as a flash.
            let hover_f = slot.press.hover.current(now);
            let active_f = slot.active.current(now);

            let label_color = color::ease(
                color::ease(color::TEXT_SECONDARY, color::TEXT_MUTED, hover_f),
                color::TEXT_PRIMARY,
                active_f,
            );
            let icon_color = color::ease(color::TEXT_SECONDARY, accent, active_f);

            let mut inner_layouts = tab_layout.children();
            let icon_layout = inner_layouts.next().expect("icon layout");
            let label_layout = inner_layouts.next().expect("label layout");
            let icon_tree = child_trees.next().expect("icon tree");
            let label_tree = child_trees.next().expect("label tree");

            item.icon.as_widget::<Message>().draw(
                icon_tree,
                renderer,
                theme,
                &RendererStyle {
                    text_color: icon_color,
                },
                icon_layout,
                cursor,
                viewport,
            );
            item.label.as_widget::<Message>().draw(
                label_tree,
                renderer,
                theme,
                &RendererStyle {
                    text_color: label_color,
                },
                label_layout,
                cursor,
                viewport,
            );

            // Underline. Width and alpha are blended off the same two
            // factors so the half-bar hover and the full-bar active state
            // share one geometry and crossfade smoothly between them.
            let bar_fraction = lerp(hover_f * HOVER_BAR_FRACTION, 1.0, active_f);
            let bar_alpha =
                lerp(color::srgb_alpha(HOVER_BAR_ALPHA) * hover_f, 1.0, active_f);
            if bar_fraction <= 0.0 || bar_alpha <= EPSILON {
                continue;
            }

            let outer = slot.last_outer;
            let full_width = outer.width - self.padding.left - self.padding.right;
            if full_width <= 0.0 {
                continue;
            }
            let bar_width = full_width * bar_fraction;
            let bar_x = outer.x + self.padding.left + (full_width - bar_width) / 2.0;

            let underline = Rectangle {
                x: strip.x + bar_x,
                y: strip.y + outer.y + outer.height,
                width: bar_width,
                height: UNDERLINE_THICKNESS,
            };

            renderer.fill_quad(
                Quad {
                    bounds: underline,
                    border: Border {
                        radius: UNDERLINE_RADIUS.into(),
                        ..Border::default()
                    },
                    ..Quad::default()
                },
                color::with_alpha(accent, bar_alpha),
            );
        }
    }

    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<TabsState>()
    }

    fn state(&self) -> tree::State {
        let target = self.selected.min(self.items.len().saturating_sub(1));
        let tabs = (0..self.items.len())
            .map(|i| TabSlot::settled(i == target))
            .collect();

        tree::State::new(TabsState {
            tabs,
            last_selected: target,
        })
    }

    fn children(&self) -> Vec<Tree> {
        self.flat_children().map(Tree::new).collect()
    }

    fn diff(&self, tree: &mut Tree) {
        let children: Vec<&dyn Widget<Message, iced::Theme, iced::Renderer>> =
            self.flat_children().collect();
        tree.diff_children(&children);

        let state = tree.state.downcast_mut::<TabsState>();
        if state.tabs.len() != self.items.len() {
            state
                .tabs
                .resize_with(self.items.len(), || TabSlot::settled(false));
        }
        if state.tabs.is_empty() {
            return;
        }

        let target = self.selected.min(state.tabs.len() - 1);
        if target == state.last_selected {
            return;
        }

        // Flip the leaving and arriving tabs on their own `active` tracks.
        // Each track keeps its eased `from` so a click landing mid-flight
        // continues from the live width rather than snapping.
        let now = Instant::now();
        if let Some(prev) = state.tabs.get_mut(state.last_selected) {
            prev.active.flip(false, now);
        }
        if let Some(next) = state.tabs.get_mut(target) {
            next.active.flip(true, now);
        }
        state.last_selected = target;
    }

    fn operate(
        &mut self,
        tree: &mut Tree,
        layout: Layout<'_>,
        renderer: &iced::Renderer,
        operation: &mut dyn Operation,
    ) {
        let mut child_trees = tree.children.iter_mut();

        for (item, tab_layout) in self.items.iter_mut().zip(layout.children()) {
            let mut inner_layouts = tab_layout.children();
            let icon_layout = inner_layouts.next().expect("icon layout");
            let label_layout = inner_layouts.next().expect("label layout");
            let icon_tree = child_trees.next().expect("icon tree");
            let label_tree = child_trees.next().expect("label tree");

            item.icon.as_widget_mut::<Message>().operate(
                icon_tree,
                icon_layout,
                renderer,
                operation,
            );
            item.label.as_widget_mut::<Message>().operate(
                label_tree,
                label_layout,
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
        // Forward to children first so any nested interactive widget can
        // claim capture before the tab strip dispatches.
        {
            let mut child_trees = tree.children.iter_mut();
            for (item, tab_layout) in self.items.iter_mut().zip(layout.children()) {
                let mut inner_layouts = tab_layout.children();
                let icon_layout = inner_layouts.next().expect("icon layout");
                let label_layout = inner_layouts.next().expect("label layout");
                let icon_tree = child_trees.next().expect("icon tree");
                let label_tree = child_trees.next().expect("label tree");

                item.icon.as_widget_mut::<Message>().update(
                    icon_tree,
                    event,
                    icon_layout,
                    cursor,
                    renderer,
                    clipboard,
                    shell,
                    viewport,
                );
                item.label.as_widget_mut::<Message>().update(
                    label_tree,
                    event,
                    label_layout,
                    cursor,
                    renderer,
                    clipboard,
                    shell,
                    viewport,
                );
            }
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
                    slot.press.press(over);
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
                    if slot.press.release(over) && dispatch_index.is_none() {
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
                    let still = state
                        .tabs
                        .iter()
                        .any(|s| s.press.animating(now) || s.active.animating(now));
                    if still {
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
        let mut child_trees = tree.children.iter();

        for (item, tab_layout) in self.items.iter().zip(layout.children()) {
            let mut inner_layouts = tab_layout.children();
            let icon_layout = inner_layouts.next().expect("icon layout");
            let label_layout = inner_layouts.next().expect("label layout");
            let icon_tree = child_trees.next().expect("icon tree");
            let label_tree = child_trees.next().expect("label tree");

            for (el, child_tree, child_layout) in [
                (&item.icon, icon_tree, icon_layout),
                (&item.label, label_tree, label_layout),
            ] {
                let inner = el.as_widget::<Message>().mouse_interaction(
                    child_tree,
                    child_layout,
                    cursor,
                    viewport,
                    renderer,
                );
                if !matches!(inner, mouse::Interaction::None | mouse::Interaction::Idle)
                {
                    return inner;
                }
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

struct TabMeasure {
    icon: (layout::Node, Size),
    label: (layout::Node, Size),
    inner_w: f32,
    inner_h: f32,
}
