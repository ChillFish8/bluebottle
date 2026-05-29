//! A clickable text element with a hover-animated underline.
//!
//! A `link` wraps one of the typography styles from [`crate::text`] and adds a
//! hover underline plus press dispatch. The underline matches the wrapped
//! text's colour. Releasing over it publishes the link's message. Inert text
//! should use [`crate::text`] directly.

use std::time::Instant;

use iced::advanced::renderer::{Quad, Style};
use iced::advanced::widget::{Operation, Tree, tree};
use iced::advanced::{Clipboard, Layout, Renderer, Shell, Widget, layout};
use iced::{Color, Element, Event, Length, Rectangle, Size, border, mouse, window};

use crate::animate::hover::{EPSILON, PressState};
use crate::color;
use crate::widget::text::Text;

/// Thickness of the hover underline, in logical pixels.
const UNDERLINE_THICKNESS: f32 = 1.0;

/// Creates a clickable link from a typography style. `on_press` is required,
/// every link is interactive.
pub fn link<'a, Message>(content: Text<'a>, on_press: Message) -> Link<'a, Message>
where
    Message: Clone + 'a,
{
    Link {
        color: content.text_color(),
        content: content.into(),
        on_press,
    }
}

/// A clickable text element, built by [`link`].
pub struct Link<'a, Message> {
    content: Element<'a, Message>,
    // The wrapped text's colour, if it set one. `None` rides the cascade, the
    // same fallback the underline uses at draw time so the two stay matched.
    color: Option<Color>,
    on_press: Message,
}

impl<'a, Message> From<Link<'a, Message>> for Element<'a, Message>
where
    Message: Clone + 'a,
{
    fn from(link: Link<'a, Message>) -> Self {
        Element::new(link)
    }
}

impl<'a, Message> Widget<Message, iced::Theme, iced::Renderer> for Link<'a, Message>
where
    Message: Clone + 'a,
{
    fn size(&self) -> Size<Length> {
        self.content.as_widget().size()
    }

    fn layout(
        &mut self,
        tree: &mut Tree,
        renderer: &iced::Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        self.content.as_widget_mut().layout(
            tree.children.first_mut().expect("link child tree"),
            renderer,
            limits,
        )
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
        self.content.as_widget().draw(
            tree.children.first().expect("link child tree"),
            renderer,
            theme,
            style,
            layout,
            cursor,
            viewport,
        );

        // Underline animates from 0 to full text width as the hover factor
        // settles to 1, in the text's colour.
        let state = tree.state.downcast_ref::<PressState>();
        let factor = state.hover.current(Instant::now());
        if factor <= EPSILON {
            return;
        }

        let bounds = layout.bounds();
        let width = bounds.width * factor;
        if width <= 0.0 {
            return;
        }

        let line = Rectangle {
            x: bounds.x,
            y: bounds.y + bounds.height,
            width,
            height: UNDERLINE_THICKNESS,
        };

        let underline = self.color.unwrap_or(style.text_color);
        renderer.fill_quad(
            Quad {
                bounds: line,
                border: border::rounded(UNDERLINE_THICKNESS / 2.0),
                ..Quad::default()
            },
            color::fade(underline, factor),
        );
    }

    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<PressState>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(PressState::default())
    }

    fn children(&self) -> Vec<Tree> {
        vec![Tree::new(&self.content)]
    }

    fn diff(&self, tree: &mut Tree) {
        tree.diff_children(std::slice::from_ref(&self.content));
    }

    fn operate(
        &mut self,
        tree: &mut Tree,
        layout: Layout<'_>,
        renderer: &iced::Renderer,
        operation: &mut dyn Operation,
    ) {
        self.content.as_widget_mut().operate(
            tree.children.first_mut().expect("link child tree"),
            layout,
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
        self.content.as_widget_mut().update(
            tree.children.first_mut().expect("link child tree"),
            event,
            layout,
            cursor,
            renderer,
            clipboard,
            shell,
            viewport,
        );

        let now = Instant::now();
        let over = cursor.is_over(layout.bounds());
        let state = tree.state.downcast_mut::<PressState>();

        match event {
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) => {
                if !shell.is_event_captured() && state.press(over) {
                    shell.capture_event();
                }
            },

            Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)) => {
                let dispatch = state.release(over);
                if dispatch && !shell.is_event_captured() {
                    shell.publish(self.on_press.clone());
                    shell.capture_event();
                }
            },

            _ => {
                // Reconcile on every other event, not just CursorMoved. A
                // scroll or layout shift can move the link out from under a
                // stationary cursor without iced emitting CursorMoved.
                if state.reconcile(over, now) {
                    shell.request_redraw();
                }
                if let Event::Window(window::Event::RedrawRequested(_)) = event
                    && state.animating(now)
                {
                    shell.request_redraw();
                }
            },
        }
    }

    fn mouse_interaction(
        &self,
        _tree: &Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        _viewport: &Rectangle,
        _renderer: &iced::Renderer,
    ) -> mouse::Interaction {
        if cursor.is_over(layout.bounds()) {
            mouse::Interaction::Pointer
        } else {
            mouse::Interaction::None
        }
    }
}
