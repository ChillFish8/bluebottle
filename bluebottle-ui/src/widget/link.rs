//! A clickable text element with a hover-animated underline.
//!
//! A `link` is a text button. Hovering animates an underline in beneath the
//! text. Pressing the link (mouse held over it) tints the text and underline
//! to [`color::primary()`]. Releasing over it publishes the link's message.
//! Inert text should use [`iced::widget::text`] instead.

use std::time::Instant;

use iced::advanced::renderer::{Quad, Style};
use iced::advanced::text::paragraph::Plain;
use iced::advanced::widget::{Tree, tree};
use iced::advanced::{Clipboard, Layout, Renderer, Shell, Widget, layout};
use iced::widget::text::{
    Alignment,
    Format,
    IntoFragment,
    LineHeight,
    Shaping,
    Style as TextStyle,
    Wrapping,
    draw as text_draw,
    layout as text_layout,
};
use iced::{
    Color,
    Element,
    Event,
    Font,
    Length,
    Pixels,
    Rectangle,
    Size,
    alignment,
    border,
    mouse,
    window,
};

use crate::animate::hover::{EPSILON, PressState};
use crate::{color, font};

/// Thickness of the hover underline, in logical pixels.
const UNDERLINE_THICKNESS: f32 = 1.0;

/// The concrete paragraph type backing iced's default renderer.
type LinkParagraph = <iced::Renderer as iced::advanced::text::Renderer>::Paragraph;

/// Creates a clickable link rendering `content`. `on_press` is required, every
/// link is interactive.
pub fn link<'a, Message>(
    content: impl IntoFragment<'a>,
    on_press: Message,
) -> Link<'a, Message>
where
    Message: Clone + 'a,
{
    Link {
        content: content.into_fragment(),
        size: Pixels(font::TEXT_MEDIUM),
        font: None,
        color: color::TEXT_PRIMARY,
        width: Length::Shrink,
        height: Length::Shrink,
        on_press,
    }
}

/// A configurable clickable text element, built by [`link`].
pub struct Link<'a, Message> {
    content: iced::widget::text::Fragment<'a>,
    size: Pixels,
    font: Option<Font>,
    color: Color,
    width: Length,
    height: Length,
    on_press: Message,
}

impl<'a, Message> Link<'a, Message>
where
    Message: Clone + 'a,
{
    /// Sets the text size. Defaults to [`font::TEXT_MEDIUM`].
    pub fn size(mut self, size: impl Into<Pixels>) -> Self {
        self.size = size.into();
        self
    }

    /// Sets the text font. Defaults to the renderer's default font.
    pub fn font(mut self, font: Font) -> Self {
        self.font = Some(font);
        self
    }

    /// Sets the idle text colour. The pressed state always shows
    /// [`color::primary()`]. Defaults to [`color::TEXT_PRIMARY`].
    pub fn color(mut self, color: Color) -> Self {
        self.color = color;
        self
    }

    /// Sets the width of the link's bounding box.
    pub fn width(mut self, width: impl Into<Length>) -> Self {
        self.width = width.into();
        self
    }

    /// Sets the height of the link's bounding box.
    pub fn height(mut self, height: impl Into<Length>) -> Self {
        self.height = height.into();
        self
    }
}

impl<'a, Message> From<Link<'a, Message>> for Element<'a, Message>
where
    Message: Clone + 'a,
{
    fn from(link: Link<'a, Message>) -> Self {
        Element::new(link)
    }
}

#[derive(Default)]
struct State {
    press: PressState,
    paragraph: Plain<LinkParagraph>,
    /// Measured size of the rendered text. Cached at layout time so draw can
    /// position the underline without re-measuring the paragraph each frame.
    text_size: Size,
}

impl<'a, Message> Widget<Message, iced::Theme, iced::Renderer> for Link<'a, Message>
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
        let state = tree.state.downcast_mut::<State>();
        // Links are single-line text buttons. Disabling wrapping means a long
        // link in a narrow container overflows or is clipped rather than
        // wrapping into a block the underline cannot meaningfully track.
        let format = Format::<Font> {
            width: self.width,
            height: self.height,
            size: Some(self.size),
            font: self.font,
            line_height: LineHeight::default(),
            align_x: Alignment::Default,
            align_y: alignment::Vertical::Top,
            shaping: Shaping::default(),
            wrapping: Wrapping::None,
        };
        let node = text_layout(
            &mut state.paragraph,
            renderer,
            limits,
            &self.content,
            format,
        );
        state.text_size = state.paragraph.min_bounds();
        node
    }

    fn draw(
        &self,
        tree: &Tree,
        renderer: &mut iced::Renderer,
        _theme: &iced::Theme,
        defaults: &Style,
        layout: Layout<'_>,
        _cursor: mouse::Cursor,
        viewport: &Rectangle,
    ) {
        let state = tree.state.downcast_ref::<State>();
        let now = Instant::now();
        let bounds = layout.bounds();

        // Press factor eases the text from its idle colour toward PRIMARY
        // while the user holds the mouse down on the link, and back when the
        // press ends (release, drag-off, or hover-off mid-press).
        let press_factor = state.press.press.current(now);
        let active_color = color::ease(self.color, color::primary(), press_factor);

        text_draw(
            renderer,
            defaults,
            bounds,
            state.paragraph.raw(),
            TextStyle {
                color: Some(active_color),
            },
            viewport,
        );

        // Underline animates from 0 to full text width as the hover factor
        // settles to 1. The colour tracks the active text colour so a pressed
        // link's underline is also primary.
        let factor = state.press.hover.current(now);
        if factor <= EPSILON {
            return;
        }

        let underline_width = state.text_size.width.min(bounds.width) * factor;
        if underline_width <= 0.0 {
            return;
        }

        let line = Rectangle {
            x: bounds.x,
            y: bounds.y + state.text_size.height,
            width: underline_width,
            height: UNDERLINE_THICKNESS,
        };

        renderer.fill_quad(
            Quad {
                bounds: line,
                border: border::rounded(UNDERLINE_THICKNESS / 2.0),
                ..Quad::default()
            },
            color::fade(active_color, factor),
        );
    }

    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<State>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(State::default())
    }

    fn update(
        &mut self,
        tree: &mut Tree,
        event: &Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        _renderer: &iced::Renderer,
        _clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
        _viewport: &Rectangle,
    ) {
        let now = Instant::now();
        let bounds = layout.bounds();
        let over = cursor.is_over(bounds);
        let state = tree.state.downcast_mut::<State>();

        match event {
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) => {
                if !shell.is_event_captured() && state.press.press(over, now) {
                    shell.capture_event();
                    shell.request_redraw();
                }
            },

            Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)) => {
                // Peek `pressed` before `release` clears it so we only
                // redraw if this link was actually in a press cycle.
                let was_pressed = state.press.pressed;
                let dispatch = state.press.release(over, now);
                if was_pressed {
                    shell.request_redraw();
                }
                if dispatch && !shell.is_event_captured() {
                    shell.publish(self.on_press.clone());
                    shell.capture_event();
                }
            },

            _ => {
                // Reconcile on every other event, not just CursorMoved. A
                // scroll or layout shift can move the link out from under
                // a stationary cursor without iced emitting CursorMoved.
                // Run before the capture gate so a sibling that claims the
                // event cannot strand the link with a stale tint or
                // underline.
                if state.press.reconcile(over, now) {
                    shell.request_redraw();
                }
                if let Event::Window(window::Event::RedrawRequested(_)) = event
                    && state.press.animating(now)
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
