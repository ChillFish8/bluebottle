//! A vertical scroll area with a Firefox-style overlay scrollbar.
//!
//! The bar is transparent at rest. It reveals while scrolling and fades out
//! after a pause. Hovering its edge widens it into a grabbable thumb. The
//! widget drives its own fade and widen animations, so the application does
//! not tick it.
//!
//! Beyond the bar the widget also supports animated programmatic scroll via
//! [`Scrollable::scroll_to`] and a user-scroll callback via
//! [`Scrollable::on_scroll`]. Content that grows or shrinks underneath the
//! viewport keeps the user's perceived position thanks to a one-anchor
//! reflow check on every frame.

use std::time::Instant;

use iced::advanced::widget::{Operation, Tree, tree};
use iced::advanced::{Clipboard, Layout, Renderer, Shell, Widget, layout, renderer};
use iced::{
    Element,
    Event,
    Length,
    Rectangle,
    Size,
    Transformation,
    Vector,
    mouse,
    window,
};

use crate::widget::scroll::ScrollEngine;

/// Makes `content` scroll vertically on overflow.
pub fn scrollable<'a, Message>(
    content: impl Into<Element<'a, Message>>,
) -> Scrollable<'a, Message> {
    Scrollable {
        content: content.into(),
        width: Length::Fill,
        height: Length::Fill,
        scroll_to: None,
        on_scroll: None,
    }
}

/// A vertical scroll area, built by [`scrollable`].
pub struct Scrollable<'a, Message> {
    content: Element<'a, Message>,
    width: Length,
    height: Length,
    scroll_to: Option<f32>,
    on_scroll: Option<Box<dyn Fn(f32) -> Message + 'a>>,
}

impl<'a, Message> Scrollable<'a, Message> {
    /// Sets the width of the scroll area.
    pub fn width(mut self, width: impl Into<Length>) -> Self {
        self.width = width.into();
        self
    }

    /// Sets the height of the scroll area.
    pub fn height(mut self, height: impl Into<Length>) -> Self {
        self.height = height.into();
        self
    }

    /// Animates the offset toward a pixel position. Passing `None` cancels
    /// any active animation and leaves the offset where it is.
    pub fn scroll_to(mut self, offset: Option<f32>) -> Self {
        self.scroll_to = offset;
        self
    }

    /// Called with the new offset after the user moves the scroll position.
    /// Programmatic scrolls from [`Scrollable::scroll_to`] do not fire it.
    pub fn on_scroll(mut self, on_scroll: impl Fn(f32) -> Message + 'a) -> Self {
        self.on_scroll = Some(Box::new(on_scroll));
        self
    }
}

impl<'a, Message> From<Scrollable<'a, Message>> for Element<'a, Message>
where
    Message: 'a,
{
    fn from(scrollable: Scrollable<'a, Message>) -> Self {
        Element::new(scrollable)
    }
}

#[derive(Clone, Copy, Default)]
struct State {
    engine: ScrollEngine,
    /// Previous frame's `scroll_to`, used to detect a fresh request.
    last_request: Option<f32>,
}

impl<'a, Message> Widget<Message, iced::Theme, iced::Renderer>
    for Scrollable<'a, Message>
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

        let child = self.content.as_widget_mut().layout(
            &mut tree.children[0],
            renderer,
            &layout::Limits::new(Size::ZERO, Size::new(viewport.width, f32::INFINITY)),
        );

        layout::Node::with_children(viewport, vec![child])
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
        let child = layout.children().next().unwrap();
        let content_height = child.bounds().height;

        let state = tree.state.downcast_ref::<State>();
        let offset = state.engine.offset;
        let shift = Transformation::translate(0.0, offset);

        renderer.with_layer(bounds, |renderer| {
            renderer.with_translation(Vector::new(0.0, -offset), |renderer| {
                self.content.as_widget().draw(
                    &tree.children[0],
                    renderer,
                    theme,
                    style,
                    child,
                    cursor * shift,
                    &(bounds * shift),
                );
            });
        });

        state
            .engine
            .bar
            .draw(renderer, bounds, content_height, offset);
    }

    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<State>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(State::default())
    }

    fn children(&self) -> Vec<Tree> {
        vec![Tree::new(&self.content)]
    }

    fn diff(&self, tree: &mut Tree) {
        tree.diff_children(std::slice::from_ref(&self.content));

        let state = tree.state.downcast_mut::<State>();
        if state.last_request != self.scroll_to {
            state.last_request = self.scroll_to;
            match self.scroll_to {
                Some(_) => state.engine.start_target(Instant::now()),
                None => state.engine.clear_target(),
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
        let child = layout.children().next().unwrap();

        self.content.as_widget_mut().operate(
            &mut tree.children[0],
            child,
            renderer,
            operation,
        );
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
        let child = layout.children().next().unwrap();
        let content_height = child.bounds().height;
        let max_offset = (content_height - bounds.height).max(0.0);

        let now = Instant::now();
        let state = tree.state.downcast_mut::<State>();

        state.engine.clamp(max_offset);

        let bar_event = state.engine.bar.update(
            event,
            bounds,
            content_height,
            state.engine.offset,
            cursor,
            shell,
        );
        let captured = bar_event.captured();

        let moved = state.engine.apply_bar_event(bar_event, max_offset, now);

        if moved && let Some(on_scroll) = &self.on_scroll {
            shell.publish(on_scroll(state.engine.offset));
        }

        if let Event::Window(window::Event::RedrawRequested(_)) = event {
            if let Some(target) = self.scroll_to {
                state.engine.step_target(now, target, max_offset);
            }
            if state.engine.animating(now) {
                shell.request_redraw();
            }
        }

        if captured {
            return;
        }

        let shift = Transformation::translate(0.0, state.engine.offset);

        self.content.as_widget_mut().update(
            &mut tree.children[0],
            event,
            child,
            cursor * shift,
            renderer,
            clipboard,
            shell,
            &(*viewport * shift),
        );
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
        let child = layout.children().next().unwrap();
        let content_height = child.bounds().height;

        let state = tree.state.downcast_ref::<State>();
        let offset = state.engine.offset;

        if let Some(interaction) =
            state
                .engine
                .bar
                .mouse_interaction(bounds, content_height, offset, cursor)
        {
            return interaction;
        }

        let shift = Transformation::translate(0.0, offset);

        self.content.as_widget().mouse_interaction(
            &tree.children[0],
            child,
            cursor * shift,
            &(*viewport * shift),
            renderer,
        )
    }

    fn overlay<'b>(
        &'b mut self,
        tree: &'b mut Tree,
        layout: Layout<'b>,
        renderer: &iced::Renderer,
        viewport: &Rectangle,
        translation: Vector,
    ) -> Option<iced::advanced::overlay::Element<'b, Message, iced::Theme, iced::Renderer>>
    {
        let child = layout.children().next().unwrap();
        let state = tree.state.downcast_ref::<State>();
        let offset = state.engine.offset;

        self.content.as_widget_mut().overlay(
            &mut tree.children[0],
            child,
            renderer,
            viewport,
            translation - Vector::new(0.0, offset),
        )
    }
}
