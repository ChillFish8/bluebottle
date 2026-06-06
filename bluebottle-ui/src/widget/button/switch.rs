//! Switch · Bordered Glass
//!
//! The bordered glass recipe stretched into a track. Off is a white glass
//! pill behind a hairline. Hover brightens the fill without moving anything.
//! On takes the accent recipe, a tinted glass behind a full accent ring.
//! The knob is its own glass disc so the moving part reads as part of the
//! same family rather than an opaque thumb breaking the chassis. The knob
//! never fades. It slides the whole on or off transition on a 220 ms
//! emphasised curve, and lights up a little brighter as it lands on the on
//! side.
//!
//! Use for instant-apply binary settings. For staged multi-select use the
//! bordered glass checkbox instead.

use std::cell::Cell;
use std::time::{Duration, Instant};

use iced::advanced::renderer::Quad;
use iced::advanced::widget::{Tree, tree};
use iced::advanced::{Clipboard, Layout, Renderer, Shell, Widget, layout, renderer};
use iced::{Border, Color, Element, Event, Length, Rectangle, Size, mouse, window};

use crate::animate::hover::{Hover, PressState};
use crate::color;
use crate::util::lerp;

/// How long the on or off transition takes. Drives the knob slide, the track
/// fill crossfade, the ring crossfade, and the knob's brightness step. Picked
/// to match the animated tick's stroke duration so settings rows that pair a
/// checkbox and a switch read as one family.
const SLIDE_DURATION: Duration = Duration::from_millis(220);

/// Inset of the knob from the track edge. Lands the knob flush inside the
/// rounded cap at both ends without overshooting the hairline.
const KNOB_INSET: f32 = 4.0;

/// Knob hairline opacity, sRGB.
const KNOB_BORDER_ALPHA: f32 = 0.40;

/// Knob fill opacity off, sRGB.
const KNOB_FILL_OFF: f32 = 0.20;

/// Knob fill opacity on, sRGB. The knob catches a little more light as the
/// switch lands so the on state reads as lit rather than just shifted.
const KNOB_FILL_ON: f32 = 0.32;

/// Size variants of the switch.
#[derive(Debug, Eq, PartialEq, Copy, Clone, Default)]
pub enum SwitchSizeVariant {
    /// 48 × 28 track with a 20 px knob.
    #[default]
    Main,
    /// 40 × 24 track with a 16 px knob.
    Alt,
}

impl SwitchSizeVariant {
    fn dimensions(self) -> (f32, f32, f32) {
        match self {
            SwitchSizeVariant::Main => (48.0, 28.0, 20.0),
            SwitchSizeVariant::Alt => (40.0, 24.0, 16.0),
        }
    }
}

/// Builds a bordered glass switch. A `None` message is the disabled state,
/// inert to hover and clicks while still painting the on or off recipe so a
/// row of settings stays visually coherent.
pub fn switch<'a, Message>(
    on: bool,
    size: SwitchSizeVariant,
    message: Option<Message>,
) -> Switch<'a, Message>
where
    Message: Clone + 'a,
{
    Switch {
        on,
        size,
        message,
        _marker: std::marker::PhantomData,
    }
}

/// The widget returned by [`switch`].
pub struct Switch<'a, Message> {
    on: bool,
    size: SwitchSizeVariant,
    message: Option<Message>,
    _marker: std::marker::PhantomData<&'a ()>,
}

impl<Message> Switch<'_, Message> {
    fn interactive(&self) -> bool {
        self.message.is_some()
    }
}

struct State {
    slide: Hover,
    press: PressState,
    last_on: bool,
    was_animating: Cell<bool>,
}

impl<'a, Message> Widget<Message, iced::Theme, iced::Renderer> for Switch<'a, Message>
where
    Message: Clone + 'a,
{
    fn size(&self) -> Size<Length> {
        let (w, h, _) = self.size.dimensions();
        Size {
            width: Length::Fixed(w),
            height: Length::Fixed(h),
        }
    }

    fn layout(
        &mut self,
        _tree: &mut Tree,
        _renderer: &iced::Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        let (w, h, _) = self.size.dimensions();
        let bounded = limits
            .width(Length::Fixed(w))
            .height(Length::Fixed(h))
            .resolve(Length::Fixed(w), Length::Fixed(h), Size::new(w, h));
        layout::Node::new(bounded)
    }

    fn draw(
        &self,
        tree: &Tree,
        renderer: &mut iced::Renderer,
        _theme: &iced::Theme,
        _style: &renderer::Style,
        layout: Layout<'_>,
        _cursor: mouse::Cursor,
        _viewport: &Rectangle,
    ) {
        let state = tree.state.downcast_ref::<State>();
        let now = Instant::now();
        let slide = state.slide.current(now);
        // The hover veil only animates while the switch can be interacted with.
        // A disabled switch reads as a static on or off pill.
        let hover = if self.interactive() {
            state.press.hover.current(now)
        } else {
            0.0
        };

        let bounds = layout.bounds();
        let (_, _, knob_size) = self.size.dimensions();

        // The off-state hover veil recedes as the on fill takes over so the
        // two translucent layers do not stack into a washed-out blend.
        let glass_factor = hover * (1.0 - slide);

        let pill = Border {
            radius: crate::border::ROUNDED_FULL.into(),
            ..Border::default()
        };

        let off_fill =
            color::ease(color::hover_veil(), color::border_strong(), glass_factor);
        let track_fill = color::ease(off_fill, color::primary_glass(), slide);

        renderer.fill_quad(
            Quad {
                bounds,
                border: pill,
                ..Quad::default()
            },
            track_fill,
        );

        // The ring rides its own quad over the fill. iced paints the border
        // band in place of the background, so layering keeps the hairline
        // visible over a translucent fill.
        let track_border = color::ease(color::border_strong(), color::primary(), slide);
        renderer.fill_quad(
            Quad {
                bounds,
                border: Border {
                    width: 1.0,
                    color: track_border,
                    ..pill
                },
                ..Quad::default()
            },
            Color::TRANSPARENT,
        );

        let off_x = bounds.x + KNOB_INSET;
        let on_x = bounds.x + bounds.width - KNOB_INSET - knob_size;
        let knob_bounds = Rectangle {
            x: lerp(off_x, on_x, slide),
            y: bounds.y + (bounds.height - knob_size) * 0.5,
            width: knob_size,
            height: knob_size,
        };

        let knob_alpha = color::srgb_alpha(lerp(KNOB_FILL_OFF, KNOB_FILL_ON, slide));
        renderer.fill_quad(
            Quad {
                bounds: knob_bounds,
                border: pill,
                ..Quad::default()
            },
            color::with_alpha(color::WHITE, knob_alpha),
        );

        renderer.fill_quad(
            Quad {
                bounds: knob_bounds,
                border: Border {
                    width: 1.0,
                    color: color::with_alpha(
                        color::WHITE,
                        color::srgb_alpha(KNOB_BORDER_ALPHA),
                    ),
                    ..pill
                },
                ..Quad::default()
            },
            Color::TRANSPARENT,
        );
    }

    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<State>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(State {
            slide: Hover::settled(self.on).with_fade(SLIDE_DURATION),
            press: PressState::default(),
            last_on: self.on,
            was_animating: Cell::new(false),
        })
    }

    fn diff(&self, tree: &mut Tree) {
        let state = tree.state.downcast_mut::<State>();
        if self.on == state.last_on {
            return;
        }
        state.slide.flip(self.on, Instant::now());
        state.last_on = self.on;
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

        if !self.interactive() {
            // Disabled switches still need to keep animating an externally
            // driven on or off change, so the slide track still gets pumped.
            if let Event::Window(window::Event::RedrawRequested(_)) = event {
                let animating = state.slide.animating(now);
                if animating || state.was_animating.get() {
                    shell.request_redraw();
                }
                state.was_animating.set(animating);
            }
            return;
        }

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
                    && let Some(message) = self.message.clone()
                {
                    shell.publish(message);
                    shell.capture_event();
                }
            },

            _ => {
                if state.press.reconcile(over, now) {
                    shell.request_redraw();
                }
                if let Event::Window(window::Event::RedrawRequested(_)) = event {
                    let animating =
                        state.press.animating(now) || state.slide.animating(now);
                    // One extra redraw on the animating-to-settled edge lands
                    // every track on its exact target factor.
                    if animating || state.was_animating.get() {
                        shell.request_redraw();
                    }
                    state.was_animating.set(animating);
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
        if self.interactive() && cursor.is_over(layout.bounds()) {
            mouse::Interaction::Pointer
        } else {
            mouse::Interaction::None
        }
    }
}

impl<'a, Message: Clone + 'a> From<Switch<'a, Message>> for Element<'a, Message> {
    fn from(switch: Switch<'a, Message>) -> Self {
        Element::new(switch)
    }
}
