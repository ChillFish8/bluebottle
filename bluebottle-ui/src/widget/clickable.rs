//! A content-agnostic clickable region with an eased hover-tint.
//!
//! Wraps any [`Element`] and adds press dispatch plus the design system's
//! 100 ms hover animation. The tint quad fades in behind the content as
//! the cursor enters. Without `on_press` the widget is fully inert. No
//! affordances animate, no pointer cursor, no event capture.
//!
//! The content colour rides on iced's cascading `text_color`. Wrapped
//! content that sets an explicit `.color(...)` on its text or icons will
//! ignore the cascade and stay at that fixed colour. To set the base tone
//! leave the content's colour unset and use [`Clickable::resting_color`]
//! to override the inherited one.
//!
//! The wrapped content is intended to be a renderer (text, icon, row of
//! both). Nesting an interactive widget that itself publishes a message
//! on release composes the two messages on a single click. Wrap the
//! interactive widget directly instead of layering it inside `clickable`.

use std::time::{Duration, Instant};

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

use crate::animate::hover::{EPSILON, FADE, Hover, PressState};
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
        tint: color::HOVER,
        resting_color: None,
        background: None,
        glow: false,
        selected: false,
        selected_background: None,
        selected_border: None,
        selected_color: None,
        selected_fade: FADE,
        border: None,
        hover_border: None,
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
    background: Option<Color>,
    glow: bool,
    selected: bool,
    selected_background: Option<Color>,
    selected_border: Option<Color>,
    selected_color: Option<Color>,
    selected_fade: Duration,
    border: Option<Color>,
    hover_border: Option<Color>,
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

    /// Sets the hover-tint colour. Defaults to [`color::HOVER`].
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

    /// Paints a solid fill behind the content, sharing the same rounded
    /// bounds as the hover tint. Without one the clickable is transparent.
    pub fn background(mut self, color: Color) -> Self {
        self.background = Some(color);
        self
    }

    /// Adds the hero glow in the background colour. It rests at a soft spread
    /// and grows on hover. No-op without a background.
    pub fn glow(mut self) -> Self {
        self.glow = true;
        self
    }

    /// Marks the clickable as on. The selected fill, ring, and colour ease in
    /// while on and out when it clears, and the hover glass fades out as the
    /// selected fill takes over.
    pub fn selected(mut self, selected: bool) -> Self {
        self.selected = selected;
        self
    }

    /// The fill shown while [`Self::selected`] is on. Eased in and out.
    pub fn selected_background(mut self, color: Color) -> Self {
        self.selected_background = Some(color);
        self
    }

    /// The 1px ring shown while [`Self::selected`] is on. Eased in and out.
    pub fn selected_border(mut self, color: Color) -> Self {
        self.selected_border = Some(color);
        self
    }

    /// The text and icon colour while [`Self::selected`] is on, eased from the
    /// resting colour. The wrapped content must defer to the cascade for this
    /// to take effect.
    pub fn selected_color(mut self, color: Color) -> Self {
        self.selected_color = Some(color);
        self
    }

    /// Overrides the selected-state fade duration. Defaults to the design
    /// system's [`FADE`]. Use [`style::EMPHASIS`](crate::style::EMPHASIS) when
    /// the chassis is paired with an inner animation that runs on the longer
    /// emphasis budget, so chassis and content stay in step.
    pub fn selected_fade(mut self, fade: Duration) -> Self {
        self.selected_fade = fade;
        self
    }

    /// Adds a 1px ring shown at rest and at full strength. When the selected
    /// ring is also set it eases in over this one as the on state takes over.
    pub fn border(mut self, color: Color) -> Self {
        self.border = Some(color);
        self
    }

    /// Adds a 1px border that fades in with the hover tint. Pass the peak
    /// colour. At rest the border is transparent so the slot is held without
    /// a shift.
    pub fn hover_border(mut self, color: Color) -> Self {
        self.hover_border = Some(color);
        self
    }

    /// Sets the corner radius of the fill, hover-tint, and glow. Defaults to
    /// the design-system pill shape.
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

/// The hover and press bookkeeping plus the eased selected track.
#[derive(Default)]
struct ClickState {
    press: PressState,
    selected: Hover,
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
        let state = tree.state.downcast_ref::<ClickState>();
        let now = Instant::now();
        let bounds = layout.bounds();

        let hover_factor = if self.interactive() {
            state.press.hover.current(now)
        } else {
            0.0
        };
        let selected_factor = state.selected.current(now);
        // The hover glass recedes as the selected fill takes over. Without a
        // selected fill there is nothing to take over so the veil rides on top.
        let glass_factor = if self.selected_background.is_some() {
            hover_factor * (1.0 - selected_factor)
        } else {
            hover_factor
        };

        let pill = Border {
            radius: self.radius.into(),
            ..Border::default()
        };

        // Glow behind the fill, resting soft and growing on hover. The quad
        // fill is transparent so only the shadow shows.
        if let Some(fill) = self.background
            && self.glow
        {
            renderer.fill_quad(
                Quad {
                    bounds,
                    border: pill,
                    shadow: crate::style::hero_glow(fill, hover_factor),
                    ..Quad::default()
                },
                Color::TRANSPARENT,
            );
        }

        // The resting fill cross-fades out as the selected fill takes over so
        // the two translucent layers do not stack into a washed-out blend.
        if let Some(fill) = self.background {
            let factor = if self.selected_background.is_some() {
                1.0 - selected_factor
            } else {
                1.0
            };
            if factor > EPSILON {
                renderer.fill_quad(
                    Quad {
                        bounds,
                        border: pill,
                        ..Quad::default()
                    },
                    color::fade(fill, factor),
                );
            }
        }

        if glass_factor > EPSILON && self.tint.a > 0.0 {
            renderer.fill_quad(
                Quad {
                    bounds,
                    border: pill,
                    ..Quad::default()
                },
                color::fade(self.tint, glass_factor),
            );
        }

        // The border rides its own quad over the fill. iced paints the border
        // band in place of the background rather than over it, so a border that
        // matches the fill would vanish. Layering it keeps the hairline visible.
        if glass_factor > EPSILON
            && let Some(border_color) = self.hover_border
        {
            renderer.fill_quad(
                Quad {
                    bounds,
                    border: Border {
                        width: 1.0,
                        color: color::fade(border_color, glass_factor),
                        ..pill
                    },
                    ..Quad::default()
                },
                Color::TRANSPARENT,
            );
        }

        // Selected fill, eased in over the on state.
        if selected_factor > EPSILON
            && let Some(fill) = self.selected_background
        {
            renderer.fill_quad(
                Quad {
                    bounds,
                    border: pill,
                    ..Quad::default()
                },
                color::fade(fill, selected_factor),
            );
        }

        // Resting ring, layered over the fill on its own quad so it sits over a
        // translucent fill rather than replacing it. Cross-fades out as the
        // selected ring takes over so their antialias fringes do not stack.
        if let Some(border_color) = self.border {
            let factor = if self.selected_border.is_some() {
                1.0 - selected_factor
            } else {
                1.0
            };
            if factor > EPSILON {
                renderer.fill_quad(
                    Quad {
                        bounds,
                        border: Border {
                            width: 1.0,
                            color: color::fade(border_color, factor),
                            ..pill
                        },
                        ..Quad::default()
                    },
                    Color::TRANSPARENT,
                );
            }
        }

        // Selected ring, eased in and layered over the fill.
        if selected_factor > EPSILON
            && let Some(border_color) = self.selected_border
        {
            renderer.fill_quad(
                Quad {
                    bounds,
                    border: Border {
                        width: 1.0,
                        color: color::fade(border_color, selected_factor),
                        ..pill
                    },
                    ..Quad::default()
                },
                Color::TRANSPARENT,
            );
        }

        // The resting tone eases toward the selected tone over the on state.
        let resting = self.resting_color.unwrap_or(style.text_color);
        let text_color = match self.selected_color {
            Some(on) => color::ease(resting, on, selected_factor),
            None => resting,
        };
        let content_style = Style { text_color };

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
        tree::Tag::of::<ClickState>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(ClickState {
            selected: Hover::settled(self.selected).with_fade(self.selected_fade),
            ..ClickState::default()
        })
    }

    fn children(&self) -> Vec<Tree> {
        vec![Tree::new(&self.content)]
    }

    fn diff(&self, tree: &mut Tree) {
        tree.diff_children(std::slice::from_ref(&self.content));

        let state = tree.state.downcast_mut::<ClickState>();
        state.selected.flip(self.selected, Instant::now());
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

        let now = Instant::now();
        let bounds = layout.bounds();
        let state = tree.state.downcast_mut::<ClickState>();

        if !self.interactive() {
            // Settle interactive bookkeeping so a future enable starts clean.
            // Without this, a press latched before disable would leak into the
            // next enabled release, and a stale hover factor would paint for
            // one frame before reconcile catches up.
            state.press.pressed = false;
            if state.press.hover.current(now) > EPSILON {
                state.press.hover = Hover::default();
            }
            
            // The selected track is driven by the parent's prop, not by input,
            // so it must keep pumping redraws while it has movement left even
            // when no message can dispatch.
            if let Event::Window(window::Event::RedrawRequested(_)) = event
                && state.selected.animating(now)
            {
                shell.request_redraw();
            }
            
            return;
        }

        let over = cursor.is_over(bounds);

        match event {
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) => {
                if !shell.is_event_captured() {
                    state.press.press(over);
                }
            },

            Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)) => {
                let dispatch = state.press.release(over);
                if dispatch
                    && !shell.is_event_captured()
                    && let Some(message) = self.on_press.clone()
                {
                    shell.publish(message);
                    shell.capture_event();
                }
            },

            _ => {
                if state.press.reconcile(over, now) {
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
