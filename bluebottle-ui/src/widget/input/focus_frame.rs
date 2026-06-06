//! A focus-within shell. Wraps any content with a glass background that eases
//! from the resting hairline to the accent ring when the wrapped content is
//! considered focused.
//!
//! The focus state is observed from outside. A left press inside our bounds
//! grants focus, a press outside drops it, and Escape clears it. This does
//! not reach into iced's private text-input focus state. iced's own caret
//! tracking on the embedded `text_input` runs independently and stays in
//! sync as long as the user clicks to focus, which matches the underlying
//! behaviour we wrap.

use std::time::Instant;

use iced::advanced::renderer::{Quad, Style};
use iced::advanced::widget::{Operation, Tree, tree};
use iced::advanced::{Clipboard, Layout, Renderer, Shell, Widget, layout};
use iced::keyboard::key::Named;
use iced::widget::text_input;
use iced::{
    Background,
    Border,
    Color,
    Element,
    Event,
    Length,
    Padding,
    Rectangle,
    Size,
    keyboard,
    mouse,
    window,
};

use crate::animate::hover::{EPSILON, Hover};
use crate::border::Radius;
use crate::color;

/// Shared text-input style for every shell in this module. A disabled field
/// keeps the same chrome as enabled, but the populated value text drops to
/// [`color::TEXT_SECONDARY`] so it reads as quiet metadata. The no-entry
/// cursor and the detached input handlers carry the rest of the inert
/// affordance.
pub(super) fn text_input_style(
    _theme: &iced::Theme,
    status: text_input::Status,
) -> text_input::Style {
    let value = match status {
        text_input::Status::Disabled => color::TEXT_SECONDARY,
        _ => color::TEXT_PRIMARY,
    };

    text_input::Style {
        background: Background::Color(Color::TRANSPARENT),
        border: Border::default(),
        icon: color::TEXT_SECONDARY,
        placeholder: color::TEXT_SECONDARY,
        value,
        selection: color::primary_glass(),
    }
}

const PILL_RADIUS: f32 = 999.0;
const FIELD_RADIUS: f32 = 12.0;

/// Background recipe.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Shape {
    /// Full pill. SearchField and Stepper.
    Pill,
    /// Rounded rectangle. TextField and PasswordField.
    Field,
}

impl Shape {
    fn radius(self) -> Radius {
        match self {
            Self::Pill => Radius::new(PILL_RADIUS),
            Self::Field => Radius::new(FIELD_RADIUS),
        }
    }
}

/// Wrap `content` in a focus-tracking shell.
pub fn focus_frame<'a, Message>(
    content: impl Into<Element<'a, Message>>,
) -> FocusFrame<'a, Message>
where
    Message: 'a,
{
    FocusFrame {
        content: content.into(),
        shape: Shape::Field,
        error: false,
        disabled: false,
        padding: Padding::ZERO,
        width: Length::Shrink,
        height: Length::Shrink,
    }
}

pub struct FocusFrame<'a, Message> {
    content: Element<'a, Message>,
    shape: Shape,
    error: bool,
    disabled: bool,
    padding: Padding,
    width: Length,
    height: Length,
}

impl<'a, Message> FocusFrame<'a, Message> {
    pub fn shape(mut self, shape: Shape) -> Self {
        self.shape = shape;
        self
    }

    pub fn error(mut self, on: bool) -> Self {
        self.error = on;
        self
    }

    pub fn disabled(mut self, on: bool) -> Self {
        self.disabled = on;
        self
    }

    pub fn padding(mut self, padding: impl Into<Padding>) -> Self {
        self.padding = padding.into();
        self
    }

    pub fn width(mut self, width: impl Into<Length>) -> Self {
        self.width = width.into();
        self
    }

    pub fn height(mut self, height: impl Into<Length>) -> Self {
        self.height = height.into();
        self
    }
}

#[derive(Default)]
struct State {
    ring: Hover,
    hover: Hover,
    focused: bool,
}

impl<'a, Message> From<FocusFrame<'a, Message>> for Element<'a, Message>
where
    Message: 'a,
{
    fn from(frame: FocusFrame<'a, Message>) -> Self {
        Element::new(frame)
    }
}

impl<Message> Widget<Message, iced::Theme, iced::Renderer> for FocusFrame<'_, Message> {
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
        let state = tree.state.downcast_ref::<State>();
        let bounds = layout.bounds();
        let now = Instant::now();

        let factor = if self.error {
            1.0
        } else {
            state.ring.current(now)
        };

        let hover_factor = state.hover.current(now);
        // Disabled fields keep the same chrome tone as enabled ones. The
        // no-entry cursor and the suppressed handlers carry the inert state.
        let alpha = 1.0;

        let border = Border {
            radius: self.shape.radius(),
            ..Border::default()
        };

        renderer.fill_quad(
            Quad {
                bounds,
                border,
                ..Quad::default()
            },
            Background::Color(color::fade(color::hover_veil(), alpha)),
        );

        if hover_factor > EPSILON {
            renderer.fill_quad(
                Quad {
                    bounds,
                    border,
                    ..Quad::default()
                },
                Background::Color(color::fade(
                    color::hover_veil(),
                    hover_factor * alpha,
                )),
            );
        }

        let resting_ring = if (1.0 - factor) > EPSILON {
            Some(color::fade(color::border_strong(), (1.0 - factor) * alpha))
        } else {
            None
        };

        if let Some(ring) = resting_ring {
            renderer.fill_quad(
                Quad {
                    bounds,
                    border: Border {
                        radius: self.shape.radius(),
                        width: 1.0,
                        color: ring,
                    },
                    ..Quad::default()
                },
                Color::TRANSPARENT,
            );
        }

        let accent_color = if self.error {
            color::error()
        } else {
            color::primary()
        };

        if factor > EPSILON {
            renderer.fill_quad(
                Quad {
                    bounds,
                    border: Border {
                        radius: self.shape.radius(),
                        width: 1.0,
                        color: color::fade(accent_color, factor * alpha),
                    },
                    ..Quad::default()
                },
                Color::TRANSPARENT,
            );
        }

        let content_layout = layout.children().next().expect("focus frame child");
        self.content.as_widget().draw(
            tree.children.first().expect("focus frame child tree"),
            renderer,
            theme,
            style,
            content_layout,
            cursor,
            viewport,
        );
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
    }

    fn operate(
        &mut self,
        tree: &mut Tree,
        layout: Layout<'_>,
        renderer: &iced::Renderer,
        operation: &mut dyn Operation,
    ) {
        let content_layout = layout.children().next().expect("focus frame child");
        self.content.as_widget_mut().operate(
            tree.children.first_mut().expect("focus frame child tree"),
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
        let content_layout = layout.children().next().expect("focus frame child");

        self.content.as_widget_mut().update(
            tree.children.first_mut().expect("focus frame child tree"),
            event,
            content_layout,
            cursor,
            renderer,
            clipboard,
            shell,
            viewport,
        );

        let now = Instant::now();
        let bounds = layout.bounds();
        let over = cursor.is_over(bounds);
        let state = tree.state.downcast_mut::<State>();

        if self.disabled {
            // The visible hover and ring must follow the disabled transition
            // even when no further event arrives. A hover or ring latched on
            // just before the disable would otherwise stay raised behind the
            // alpha wash until the cursor leaves the area on its own.
            let hover_changed = state.hover.flip(false, now);
            let ring_changed = state.ring.flip(false, now);
            state.focused = false;
            if hover_changed || ring_changed {
                shell.request_redraw();
            }
            return;
        }

        match event {
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) => {
                // iced's text_input captures the press to position its caret
                // before our update sees the event, so gating on
                // `is_event_captured()` here would block legitimate focus
                // gain. We track focus by press-inside-bounds and accept the
                // minor cosmetic side effect that pressing an inner action
                // button (clear, eye) also lights the ring. The user is
                // typically about to keep editing in either case.
                state.focused = over;
                if state.ring.flip(state.focused, now) {
                    shell.request_redraw();
                }
            },

            Event::Keyboard(keyboard::Event::KeyPressed {
                key: keyboard::Key::Named(Named::Escape),
                ..
            }) if state.focused => {
                state.focused = false;
                if state.ring.flip(false, now) {
                    shell.request_redraw();
                }
            },

            Event::Window(window::Event::RedrawRequested(_))
                if state.ring.animating(now) || state.hover.animating(now) =>
            {
                shell.request_redraw();
            },

            _ => {},
        }

        if state.hover.flip(over, now) {
            shell.request_redraw();
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
        if self.disabled && cursor.is_over(layout.bounds()) {
            return mouse::Interaction::NotAllowed;
        }

        let content_layout = layout.children().next().expect("focus frame child");
        self.content.as_widget().mouse_interaction(
            tree.children.first().expect("focus frame child tree"),
            content_layout,
            cursor,
            viewport,
            renderer,
        )
    }
}
