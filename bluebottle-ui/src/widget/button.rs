//! Button widgets. Hover and press are eased by the design system's 100 ms
//! [`Hover`](crate::animate::hover::Hover) primitive rather than iced's
//! instant status-based styling. Hovering fades a pill tint in behind the
//! content. Pressing eases the text and icon colour toward `PRIMARY` (or
//! away from it for a selected `toggle_icon`). `standard`, `icon`, and
//! `toggle_icon` are thin builders over
//! [`clickable`](super::clickable::clickable). `nav` is its own custom
//! widget because its `selected` state has a second animation track that
//! cross-fades the pill behind the icon. `disabled` stays a plain iced
//! `button` since it has nothing to animate.

use std::time::Instant;

pub use button::{Status, Style};
use iced::advanced::renderer::{Quad, Style as RendererStyle};
use iced::advanced::widget::{Operation, Tree, tree};
use iced::advanced::{Clipboard, Layout, Renderer, Shell, Widget, layout};
use iced::widget::{Text, button, column, container, row, text};
use iced::{
    Border,
    Center,
    Element,
    Event,
    Length,
    Rectangle,
    Size,
    Theme,
    border,
    mouse,
    window,
};

use super::clickable::clickable;
use crate::animate::hover::{EPSILON, Hover, PressState};
use crate::{color, font, icon};

const STANDARD_PADDING: [u16; 2] = [5, 10];
const ICON_PADDING: u16 = 4;

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

/// A standard button. Optional leading icon plus label, pill background.
pub fn standard<'a, Message>(
    label: &'a str,
    icon_name: Option<&'a str>,
    selected: bool,
    message: Message,
) -> Element<'a, Message>
where
    Message: Clone + 'a,
{
    let mut items = row![].spacing(4).align_y(Center);
    if let Some(name) = icon_name {
        items = items.push(icon::filled(name).size(24));
    }
    items = items.push(text(label));

    let message = (!selected).then_some(message);
    let mut button = clickable(items)
        .padding(STANDARD_PADDING)
        .on_press_maybe(message);
    if selected {
        button = button.resting_color(color::TEXT_PRIMARY);
    }
    button.into()
}

/// A disabled button. Cannot be interacted with. Sizes like a [`standard`]
/// or [`icon`] button so it slots into the same rows without shifting.
pub fn disabled<'a, Message>(
    label: Option<&'a str>,
    icon_name: Option<&'a str>,
) -> Element<'a, Message>
where
    Message: Clone + 'a,
{
    if let (Some(name), None) = (icon_name, label) {
        let inner = icon::filled(name).size(24).color(color::TEXT_DARK);
        return button(inner)
            .padding(ICON_PADDING)
            .style(disabled_button_style)
            .into();
    }

    let mut items = row![].spacing(4).align_y(Center);

    if let Some(name) = icon_name {
        items = items.push(icon::filled(name).size(24).color(color::TEXT_DARK));
    }

    if let Some(label) = label {
        items = items.push(text(label).color(color::TEXT_DARK));
    }

    button(items).style(disabled_button_style).into()
}

#[doc(hidden)]
/// An icon name or pre-created icon text widget.
pub enum IconTextOrName<'a> {
    Name(&'a str),
    Text(Text<'a>),
}

impl<'a> From<&'a str> for IconTextOrName<'a> {
    fn from(value: &'a str) -> Self {
        Self::Name(value)
    }
}

impl<'a> From<Text<'a>> for IconTextOrName<'a> {
    fn from(value: Text<'a>) -> Self {
        Self::Text(value)
    }
}

/// An icon button. No label, only a clickable icon.
pub fn icon<'a, Message>(
    icon_input: impl Into<IconTextOrName<'a>>,
    selected: bool,
    message: Message,
) -> Element<'a, Message>
where
    Message: Clone + 'a,
{
    let inner = match icon_input.into() {
        IconTextOrName::Name(name) => icon::filled(name),
        IconTextOrName::Text(text) => text,
    };

    let message = (!selected).then_some(message);
    let mut button = clickable(inner)
        .padding(ICON_PADDING)
        .on_press_maybe(message);
    if selected {
        button = button.resting_color(color::TEXT_PRIMARY);
    }
    button.into()
}

/// An icon toggle button. The icon swaps when `selected` flips.
pub fn toggle_icon<'a, Message>(
    base_icon: &'a str,
    selected_icon: &'a str,
    selected: bool,
    message: Message,
) -> Element<'a, Message>
where
    Message: Clone + 'a,
{
    if selected {
        // Selected: icon sits at PRIMARY at rest and eases to TEXT_DEFAULT
        // on press. The cascade drives the colour, so the icon must not
        // set an explicit `.color(...)`.
        clickable(icon::filled(selected_icon))
            .padding(ICON_PADDING)
            .resting_color(color::TEXT_PRIMARY)
            .press_color(color::TEXT_DEFAULT)
            .on_press(message)
            .into()
    } else {
        clickable(icon::outline(base_icon))
            .padding(ICON_PADDING)
            .on_press(message)
            .into()
    }
}

fn disabled_button_style(_theme: &Theme, _status: Status) -> Style {
    Style {
        text_color: color::TEXT_DARK,
        background: None,
        border: border::rounded(999),
        ..Style::default()
    }
}

const NAV_ICON_PADDING: [u16; 2] = [4, 16];
const NAV_PILL_RADIUS: f32 = 28.0;

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
        // The label keeps an explicit TEXT_DEFAULT so the draw-time
        // cascade only reaches the icon glyph. The icon's colour is
        // driven entirely from `draw` via `text_color`, eased from
        // TEXT_DEFAULT toward PRIMARY by the larger of `selected` and
        // `press`. Owning the colour from `draw` (rather than baking it
        // into the icon Element here) keeps the icon in lockstep with
        // the animated pill when `selected` toggles.
        let icon_text = icon::filled(icon_name);
        let label_text = text(label)
            .size(font::TEXT_SMALL)
            .align_x(Center)
            .color(color::TEXT_DEFAULT);

        // Built once and owned so the widget tree state stays consistent
        // across frames. Rebuilding each call would hand `diff_children` a
        // fresh Element every frame and lose the animated state.
        let content: Element<'a, Message> =
            column![container(icon_text).padding(NAV_ICON_PADDING), label_text,]
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
        let press_factor = state.press.press.current(now);

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
                color::fade(color::HOVER_HIGHLIGHT, pill_factor),
            );
        }

        let content_style = RendererStyle {
            text_color: color::ease(
                color::TEXT_DEFAULT,
                color::PRIMARY,
                selected_factor.max(press_factor),
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
                if !shell.is_event_captured() && state.press.press(over, now) {
                    shell.request_redraw();
                }
            },

            Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)) => {
                // Always clear any in-flight press cycle, even when this
                // entry is now selected, so a selected flip mid-press
                // cannot leak `pressed = true` into a later click.
                let was_pressed = state.press.pressed;
                let dispatch = state.press.release(over, now);
                if was_pressed {
                    shell.request_redraw();
                }
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
