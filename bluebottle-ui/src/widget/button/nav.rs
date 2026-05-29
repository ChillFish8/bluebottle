use std::time::Instant;

use iced::advanced::renderer::{Quad, Style as RendererStyle};
use iced::advanced::widget::{Operation, Tree, tree};
use iced::advanced::{Clipboard, Layout, Renderer, Shell, Widget, layout};
use iced::widget::{column, container};
use iced::{Border, Center, Element, Event, Length, Rectangle, Size, mouse, window};

use crate::animate::hover::{EPSILON, Hover, PressState};
use crate::{color, icon, text};

const NAV_ICON_PADDING: [u16; 2] = [4, 16];
const NAV_PILL_RADIUS: f32 = 28.0;

/// A navbar button.
///
/// Icon over label, vertically centred. The pill behind the icon animates
/// both on hover (cursor enter and leave) and when `selected` toggles. The
/// content scales down briefly on press. When `selected` is true the press
/// dispatches no message so reselecting the active entry is a no-op.
pub fn nav<'a, Message>(
    label: &'a str,
    icon: &'a str,
    selected: bool,
    message: Message,
) -> Element<'a, Message>
where
    Message: Clone + 'a,
{
    Element::new(NavButton::new(label, icon, selected, message))
}

struct NavButton<'a, Message> {
    selected: bool,
    message: Message,
    content: Element<'a, Message>,
}

impl<'a, Message> NavButton<'a, Message>
where
    Message: Clone + 'a,
{
    fn new(
        label: &'a str,
        icon_name: &'a str,
        selected: bool,
        message: Message,
    ) -> Self {
        let icon_text = icon::filled(icon_name);
        let label_text = text::micro_label(label).align_x(Center);

        // Built once and owned so the widget tree state stays consistent
        // across frames. Rebuilding each call would hand `diff_children` a
        // fresh Element every frame and lose the animated state.
        let content =
            column![container(icon_text).padding(NAV_ICON_PADDING), label_text]
                .align_x(Center)
                .into();

        Self {
            selected,
            message,
            content,
        }
    }
}

#[derive(Clone, Copy, Default)]
struct NavState {
    press: PressState,
    selected: Hover,
}

fn pill_bounds(content_layout: Layout<'_>) -> Option<Rectangle> {
    let icon_container = content_layout.children().next()?;
    Some(icon_container.bounds())
}

impl<'a, Message> Widget<Message, iced::Theme, iced::Renderer> for NavButton<'a, Message>
where
    Message: Clone + 'a,
{
    fn size(&self) -> Size<Length> {
        Size {
            width: Length::Shrink,
            height: Length::Shrink,
        }
    }

    fn layout(
        &mut self,
        tree: &mut Tree,
        renderer: &iced::Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        let node = self.content.as_widget_mut().layout(
            tree.children.first_mut().expect("nav child tree"),
            renderer,
            limits,
        );
        layout::Node::with_children(node.size(), vec![node])
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
        let state = tree.state.downcast_ref::<NavState>();
        let now = Instant::now();
        let hover_factor = state.press.hover.current(now);
        let selected_factor = state.selected.current(now);

        let pill_factor = hover_factor.max(selected_factor);
        let content_layout = layout.children().next().expect("nav child layout");

        if pill_factor > EPSILON
            && let Some(pill) = pill_bounds(content_layout)
        {
            renderer.fill_quad(
                Quad {
                    bounds: pill,
                    border: Border {
                        radius: NAV_PILL_RADIUS.into(),
                        ..Border::default()
                    },
                    ..Quad::default()
                },
                color::fade(color::HOVER, pill_factor),
            );
        }

        let content_style = RendererStyle {
            text_color: color::ease(
                color::TEXT_PRIMARY,
                color::primary(),
                selected_factor,
            ),
        };

        self.content.as_widget().draw(
            tree.children.first().expect("nav child tree"),
            renderer,
            theme,
            &content_style,
            content_layout,
            cursor,
            viewport,
        );
    }

    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<NavState>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(NavState {
            selected: Hover::settled(self.selected),
            ..NavState::default()
        })
    }

    fn children(&self) -> Vec<Tree> {
        vec![Tree::new(&self.content)]
    }

    fn diff(&self, tree: &mut Tree) {
        tree.diff_children(std::slice::from_ref(&self.content));

        let state = tree.state.downcast_mut::<NavState>();
        state.selected.flip(self.selected, Instant::now());
    }

    fn operate(
        &mut self,
        tree: &mut Tree,
        layout: Layout<'_>,
        renderer: &iced::Renderer,
        operation: &mut dyn Operation,
    ) {
        let content_layout = layout.children().next().expect("nav child layout");
        self.content.as_widget_mut().operate(
            tree.children.first_mut().expect("nav child tree"),
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
        let content_layout = layout.children().next().expect("nav child layout");
        self.content.as_widget_mut().update(
            tree.children.first_mut().expect("nav child tree"),
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
        let state = tree.state.downcast_mut::<NavState>();

        match event {
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left))
                if !self.selected =>
            {
                if !shell.is_event_captured() {
                    state.press.press(over);
                }
            },

            Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)) => {
                // Always clear any in-flight press cycle, even when this
                // entry is now selected, so a selected flip mid-press
                // cannot leak `pressed = true` into a later click.
                let dispatch = state.press.release(over);
                if dispatch && !self.selected && !shell.is_event_captured() {
                    shell.publish(self.message.clone());
                    shell.capture_event();
                }
            },

            _ => {
                if !self.selected && state.press.reconcile(over, now) {
                    shell.request_redraw();
                }
                if let Event::Window(window::Event::RedrawRequested(_)) = event
                    && (state.press.animating(now) || state.selected.animating(now))
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
        let content_layout = layout.children().next().expect("nav child layout");
        let inner = self.content.as_widget().mouse_interaction(
            tree.children.first().expect("nav child tree"),
            content_layout,
            cursor,
            viewport,
            renderer,
        );
        if !matches!(inner, mouse::Interaction::None | mouse::Interaction::Idle) {
            return inner;
        }

        if !self.selected && cursor.is_over(layout.bounds()) {
            return mouse::Interaction::Pointer;
        }

        mouse::Interaction::None
    }
}
