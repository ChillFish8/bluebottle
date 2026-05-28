//! A sliding sidebar drawer that animates itself.
//!
//! It paints a veil over the screen, slides a frosted drawer in from the right,
//! and dismisses on a click on the veil. The drawer drives its own open and
//! close animation, so the application only has to set `open`.

use std::time::{Duration, Instant};

use iced::advanced::renderer::{Quad, Style};
use iced::advanced::widget::{Operation, Tree, tree};
use iced::advanced::{Clipboard, Layout, Renderer, Shell, Widget, layout};
use iced::widget::{container, space, stack};
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

use crate::widget::splash_background::{Backdrop, splash_panel};
use crate::{color, easing, style};

/// How long the drawer takes to slide in or out.
const FADE: Duration = Duration::from_millis(220);

/// The drawer's width, in logical pixels.
const DEFAULT_WIDTH: f32 = 850.0;

/// Below this slide factor the drawer counts as fully closed.
const EPSILON: f32 = 0.001;

/// A sliding sidebar over `content`, with `image` frosted behind it.
pub fn sidebar<'a, Message>(
    content: impl Into<Element<'a, Message>>,
    image: Option<Backdrop>,
) -> Sidebar<'a, Message> {
    Sidebar {
        content: content.into(),
        image,
        open: false,
        on_dismiss: None,
        width: DEFAULT_WIDTH,
    }
}

/// A configurable sidebar, built by [`sidebar`].
pub struct Sidebar<'a, Message> {
    content: Element<'a, Message>,
    image: Option<Backdrop>,
    open: bool,
    on_dismiss: Option<Message>,
    width: f32,
}

impl<'a, Message> Sidebar<'a, Message> {
    /// Opens or closes the drawer. The change is animated.
    pub fn open(mut self, open: bool) -> Self {
        self.open = open;
        self
    }

    /// Sets the message published when the veil is clicked.
    pub fn on_dismiss(mut self, message: Message) -> Self {
        self.on_dismiss = Some(message);
        self
    }

    /// Sets the drawer width, in logical pixels.
    pub fn width(mut self, width: f32) -> Self {
        self.width = width;
        self
    }
}

impl<'a, Message> From<Sidebar<'a, Message>> for Element<'a, Message>
where
    Message: Clone + 'a,
{
    fn from(sidebar: Sidebar<'a, Message>) -> Self {
        let border = container(
            space()
                .width(Length::Fixed(style::BORDER_WIDTH))
                .height(Length::Fill),
        )
        .style(|_theme| container::Style {
            background: Some(color::BORDER.into()),
            ..container::Style::default()
        });

        // The shader paints the opaque background. The container adds the
        // leading-edge elevation shadow.
        let drawer =
            container(stack![splash_panel(sidebar.image), sidebar.content, border,])
                .width(Length::Fixed(sidebar.width))
                .height(Length::Fill)
                .clip(true)
                .style(|_theme| container::Style {
                    shadow: style::PANEL_SHADOW,
                    ..container::Style::default()
                });

        Element::new(Drawer {
            content: drawer.into(),
            open: sidebar.open,
            on_dismiss: sidebar.on_dismiss,
            width: sidebar.width,
        })
    }
}

/// The animated widget behind a [`Sidebar`], wrapping the composed drawer.
struct Drawer<'a, Message> {
    content: Element<'a, Message>,
    open: bool,
    on_dismiss: Option<Message>,
    width: f32,
}

#[derive(Clone, Copy)]
struct State {
    /// Where the slide is heading, 1.0 when open and 0.0 when closed.
    target: f32,
    /// The live factor when the current move began.
    from: f32,
    /// Start of the current move.
    started: Instant,
}

impl Default for State {
    fn default() -> Self {
        Self {
            target: 0.0,
            from: 0.0,
            started: Instant::now(),
        }
    }
}

impl State {
    /// The slide factor right now, eased over `FADE`.
    fn current(&self, now: Instant) -> f32 {
        let raw = (now.duration_since(self.started).as_secs_f32() / FADE.as_secs_f32())
            .clamp(0.0, 1.0);

        let curve = if self.target >= self.from {
            &easing::EMPHASIZED_DECELERATE
        } else {
            &easing::EMPHASIZED_ACCELERATE
        };

        self.from + (self.target - self.from) * curve.y_at_x(raw)
    }
}

impl<'a, Message> Widget<Message, iced::Theme, iced::Renderer> for Drawer<'a, Message>
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
        renderer: &iced::Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        let bounds = limits.max();

        let child = self.content.as_widget_mut().layout(
            &mut tree.children[0],
            renderer,
            &layout::Limits::new(Size::ZERO, Size::new(self.width, bounds.height)),
        );

        // Dock the drawer against the right edge. The slide is applied at draw
        // and event time, so the layout stays put.
        let docked = bounds.width - child.size().width;
        let child = child.move_to(Point::new(docked, 0.0));

        layout::Node::with_children(bounds, vec![child])
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
        let factor = tree.state.downcast_ref::<State>().current(Instant::now());

        if factor <= EPSILON {
            return;
        }

        renderer.fill_quad(
            Quad {
                bounds: layout.bounds(),
                ..Quad::default()
            },
            color::with_alpha(color::VEIL, color::VEIL.a * factor),
        );

        let offset = (1.0 - factor) * self.width;
        let shift = Transformation::translate(-offset, 0.0);
        let child = layout.children().next().unwrap();

        renderer.with_translation(Vector::new(offset, 0.0), |renderer| {
            self.content.as_widget().draw(
                &tree.children[0],
                renderer,
                theme,
                style,
                child,
                cursor * shift,
                &(*viewport * shift),
            );
        });
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
        let state = tree.state.downcast_mut::<State>();

        if let Event::Window(window::Event::RedrawRequested(now)) = event {
            let target = if self.open { 1.0 } else { 0.0 };

            if target != state.target {
                state.from = state.current(*now);
                state.target = target;
                state.started = *now;
            }

            if now.duration_since(state.started) < FADE {
                shell.request_redraw();
            }
        }

        let factor = state.current(Instant::now());

        if factor <= EPSILON {
            return;
        }

        let offset = (1.0 - factor) * self.width;
        let shift = Transformation::translate(-offset, 0.0);
        let child = layout.children().next().unwrap();

        // The drawer keeps animating its own contents (the frosted shader) and
        // handling its buttons. Offset events to match the slide.
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

        if let Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) = event {
            let drawer = child.bounds() + Vector::new(offset, 0.0);

            if cursor.is_over(drawer) {
                // Swallow the press so it does not fall through the drawer.
                shell.capture_event();
            } else if let Some(message) = &self.on_dismiss {
                shell.publish(message.clone());
                shell.capture_event();
            }
        }

        // Keep the covered content from scrolling while the sidebar is open. The
        // drawer already had the event, so capturing only blocks the background.
        if let Event::Mouse(mouse::Event::WheelScrolled { .. }) = event {
            shell.capture_event();
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
        let factor = tree.state.downcast_ref::<State>().current(Instant::now());

        if factor <= EPSILON {
            return mouse::Interaction::None;
        }

        let offset = (1.0 - factor) * self.width;
        let shift = Transformation::translate(-offset, 0.0);
        let child = layout.children().next().unwrap();
        let drawer = child.bounds() + Vector::new(offset, 0.0);

        if cursor.is_over(drawer) {
            self.content.as_widget().mouse_interaction(
                &tree.children[0],
                child,
                cursor * shift,
                &(*viewport * shift),
                renderer,
            )
        } else {
            mouse::Interaction::None
        }
    }
}
