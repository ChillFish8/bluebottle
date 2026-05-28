//! A clickable text element with a hover-animated underline.
//!
//! A `link` is a text button. Hovering animates an underline in beneath the
//! text. Pressing the link (mouse held over it) tints the text and underline
//! to [`color::PRIMARY`]. Releasing over it publishes the link's message.
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

use crate::animate::hover::{EPSILON, Hover};
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
        color: color::TEXT_DEFAULT,
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
    /// [`color::PRIMARY`]. Defaults to [`color::TEXT_DEFAULT`].
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
    hover: Hover,
    /// Eased factor that tracks whether the link is currently in its pressed
    /// (held-down and hovered) state. Drives the text colour from idle toward
    /// [`color::PRIMARY`].
    press: Hover,
    /// Whether a left button press started over the link. Releases without a
    /// matching press are ignored.
    pressed: bool,
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
        let press_factor = state.press.current(now);
        let active_color = color::mix(self.color, color::PRIMARY, press_factor);

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
        let factor = state.hover.current(now);
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
            color::with_alpha(active_color, active_color.a * factor),
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

        // Reconcile the hover and press factors with the live cursor on every
        // event, not just CursorMoved. This catches the case where a scroll or
        // layout change moves the link out from under (or back under) a
        // stationary cursor without iced emitting a CursorMoved. `Hover::flip`
        // is idempotent and reports back when the target actually changes, so
        // we only request a redraw on the transition edge.
        if state.hover.flip(over, now) {
            shell.request_redraw();
        }
        if state.press.flip(state.pressed && over, now) {
            shell.request_redraw();
        }

        // Press dispatch must yield to siblings/overlays that have already
        // claimed the event. Hover/press factor updates above are deliberately
        // outside this gate so a captured-by-sibling interaction (e.g. a
        // scroll gesture mid-press) cannot strand the link with a stale
        // pressed-tint or underline.
        if shell.is_event_captured() {
            return;
        }

        match event {
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) if over => {
                state.pressed = true;
                state.press.flip(true, now);
                shell.capture_event();
                shell.request_redraw();
            },

            Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left))
                if state.pressed =>
            {
                state.pressed = false;
                state.press.flip(false, now);
                shell.request_redraw();

                if over {
                    shell.publish(self.on_press.clone());
                    shell.capture_event();
                }
            },

            Event::Window(window::Event::RedrawRequested(_))
                if state.hover.animating(now) || state.press.animating(now) =>
            {
                shell.request_redraw();
            },

            _ => {},
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
