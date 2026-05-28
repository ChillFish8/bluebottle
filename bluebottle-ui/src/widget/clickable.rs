//! A content-agnostic clickable region with eased hover-tint and press
//! text-recolour.
//!
//! Wraps any [`Element`] and adds press dispatch plus the design system's
//! 100 ms hover animation. The tint quad fades in behind the content as
//! the cursor enters. While the cursor holds down, the content's text and
//! icon glyph colour eases from its resting tone toward [`press_color`]
//! (default [`color::PRIMARY`]). Without `on_press` the widget is fully
//! inert. No affordances animate, no pointer cursor, no event capture.
//!
//! The colour animation rides on iced's cascading `text_color`. Wrapped
//! content that sets an explicit `.color(...)` on its text or icons will
//! ignore the cascade and stay at that fixed colour. To recolour on press
//! leave the content's colour unset and use [`Clickable::resting_color`]
//! to override the inherited base.
//!
//! The wrapped content is intended to be a renderer (text, icon, row of
//! both). Nesting an interactive widget that itself publishes a message
//! on release composes the two messages on a single click. Wrap the
//! interactive widget directly instead of layering it inside `clickable`.
//!
//! [`press_color`]: Clickable::press_color

use std::time::Instant;

use iced::advanced::renderer::{Quad, Style};
use iced::advanced::widget::{Operation, Tree, tree};
use iced::advanced::{Clipboard, Layout, Renderer, Shell, Widget, layout};
use iced::{
    Border,
    Color,
    Element,
    Event,
    Length,
    Padding,
    Rectangle,
    Size,
    mouse,
    window,
};

use crate::animate::hover::{EPSILON, PressState};
use crate::color;

const DEFAULT_RADIUS: f32 = 999.0;

/// Creates a clickable around `content`. Non-interactive by default. Set
/// `.on_press(...)` to enable the press dispatch and the hover affordances.
pub fn clickable<'a, Message>(
    content: impl Into<Element<'a, Message>>,
) -> Clickable<'a, Message>
where
    Message: Clone + 'a,
{
    Clickable {
        content: content.into(),
        on_press: None,
        tint: color::HOVER_HIGHLIGHT,
        resting_color: None,
        press_color: color::PRIMARY,
        radius: DEFAULT_RADIUS,
        padding: Padding::ZERO,
        width: Length::Shrink,
        height: Length::Shrink,
    }
}

/// A configurable clickable region, built by [`clickable`].
pub struct Clickable<'a, Message> {
    content: Element<'a, Message>,
    on_press: Option<Message>,
    tint: Color,
    resting_color: Option<Color>,
    press_color: Color,
    radius: f32,
    padding: Padding,
    width: Length,
    height: Length,
}

impl<'a, Message> Clickable<'a, Message>
where
    Message: Clone + 'a,
{
    /// Sets the press message. Required to enable the hover affordances and
    /// the pointer cursor. Without one the widget is inert.
    pub fn on_press(mut self, message: Message) -> Self {
        self.on_press = Some(message);
        self
    }

    /// Sets the press message from an [`Option`]. Convenience for callers
    /// that already gate dispatch on some external selected/disabled flag.
    pub fn on_press_maybe(mut self, message: Option<Message>) -> Self {
        self.on_press = message;
        self
    }

    /// Sets the hover-tint colour. Defaults to [`color::HOVER_HIGHLIGHT`].
    pub fn tint(mut self, color: Color) -> Self {
        self.tint = color;
        self
    }

    /// Overrides the text and icon colour at rest. The cascade picks this
    /// up unless the wrapped content sets its own `.color(...)`.
    pub fn resting_color(mut self, color: Color) -> Self {
        self.resting_color = Some(color);
        self
    }

    /// Sets the text and icon colour reached at full press. The resting
    /// colour eases toward this as the press factor lifts. Defaults to
    /// [`color::PRIMARY`].
    pub fn press_color(mut self, color: Color) -> Self {
        self.press_color = color;
        self
    }

    /// Sets the corner radius of the hover-tint quad. Defaults to the
    /// design-system pill shape.
    pub fn radius(mut self, radius: f32) -> Self {
        self.radius = radius;
        self
    }

    /// Sets the padding around the content. The hit area, the tint quad,
    /// and the press scale-down pivot all use the padded bounds.
    pub fn padding(mut self, padding: impl Into<Padding>) -> Self {
        self.padding = padding.into();
        self
    }

    /// Sets the width of the clickable's bounding box.
    pub fn width(mut self, width: impl Into<Length>) -> Self {
        self.width = width.into();
        self
    }

    /// Sets the height of the clickable's bounding box.
    pub fn height(mut self, height: impl Into<Length>) -> Self {
        self.height = height.into();
        self
    }

    fn interactive(&self) -> bool {
        self.on_press.is_some()
    }
}

impl<'a, Message> From<Clickable<'a, Message>> for Element<'a, Message>
where
    Message: Clone + 'a,
{
    fn from(c: Clickable<'a, Message>) -> Self {
        Element::new(c)
    }
}

impl<'a, Message> Widget<Message, iced::Theme, iced::Renderer> for Clickable<'a, Message>
where
    Message: Clone + 'a,
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
        layout::padded(limits, self.width, self.height, self.padding, |limits| {
            self.content
                .as_widget_mut()
                .layout(&mut tree.children[0], renderer, limits)
        })
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
        let state = tree.state.downcast_ref::<PressState>();
        let now = Instant::now();
        let bounds = layout.bounds();

        let (hover_factor, press_factor) = if self.interactive() {
            (state.hover.current(now), state.press.current(now))
        } else {
            (0.0, 0.0)
        };

        if hover_factor > EPSILON {
            renderer.fill_quad(
                Quad {
                    bounds,
                    border: Border {
                        radius: self.radius.into(),
                        ..Border::default()
                    },
                    ..Quad::default()
                },
                color::fade(self.tint, hover_factor),
            );
        }

        let resting = self.resting_color.unwrap_or(style.text_color);
        let content_style = Style {
            text_color: color::ease(resting, self.press_color, press_factor),
        };

        let content_layout = layout.children().next().expect("clickable child");
        self.content.as_widget().draw(
            tree.children.first().expect("clickable child tree"),
            renderer,
            theme,
            &content_style,
            content_layout,
            cursor,
            viewport,
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
        let content_layout = layout.children().next().expect("clickable child");
        self.content.as_widget_mut().operate(
            tree.children.first_mut().expect("clickable child tree"),
            content_layout,
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
        let content_layout = layout.children().next().expect("clickable child");

        // Forward to the child first so any nested widget can claim the
        // event (capture or publish) before we check the dispatch path.
        self.content.as_widget_mut().update(
            tree.children.first_mut().expect("clickable child tree"),
            event,
            content_layout,
            cursor,
            renderer,
            clipboard,
            shell,
            viewport,
        );

        if !self.interactive() {
            return;
        }

        let now = Instant::now();
        let bounds = layout.bounds();
        let over = cursor.is_over(bounds);
        let state = tree.state.downcast_mut::<PressState>();

        match event {
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) => {
                if !shell.is_event_captured() && state.press(over, now) {
                    shell.request_redraw();
                }
            },

            Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)) => {
                // Peek `pressed` before `release` clears it so we can gate
                // the redraw on this widget actually being in a press
                // cycle. Otherwise every global release floods every
                // clickable on screen with a wasted redraw.
                let was_pressed = state.pressed;
                let dispatch = state.release(over, now);
                if was_pressed {
                    shell.request_redraw();
                }
                if dispatch
                    && !shell.is_event_captured()
                    && let Some(message) = self.on_press.clone()
                {
                    shell.publish(message);
                    shell.capture_event();
                }
            },

            _ => {
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
        tree: &Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
        renderer: &iced::Renderer,
    ) -> mouse::Interaction {
        let content_layout = layout.children().next().expect("clickable child");
        let inner = self.content.as_widget().mouse_interaction(
            tree.children.first().expect("clickable child tree"),
            content_layout,
            cursor,
            viewport,
            renderer,
        );
        if !matches!(inner, mouse::Interaction::None | mouse::Interaction::Idle) {
            return inner;
        }

        if self.interactive() && cursor.is_over(layout.bounds()) {
            return mouse::Interaction::Pointer;
        }

        mouse::Interaction::None
    }
}
