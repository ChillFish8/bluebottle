//! Switch · Bordered Glass
//!
//! The bordered glass recipe stretched into a track. Off is a white glass
//! pill behind a hairline. Hover brightens the fill without moving anything.
//! On takes the accent recipe, a tinted glass behind a full accent ring.
//! The knob is its own glass disc so the moving part reads as part of the
//! same family rather than an opaque thumb breaking the chassis. The knob
//! never fades. It slides the whole on or off transition on the design
//! system's emphasis budget, and lights up a little brighter as it lands on
//! the on side.
//!
//! The chassis is a stock [`clickable`] tuned to the bordered-glass recipe,
//! with the selected fade widened to [`style::EMPHASIS`] so the track colour
//! crossfade lands in step with the knob slide. Only the knob is bespoke.
//!
//! Use for instant-apply binary settings. For staged multi-select use the
//! bordered glass checkbox instead.

use std::time::Instant;

use iced::advanced::renderer::Quad;
use iced::advanced::widget::{Tree, tree};
use iced::advanced::{Clipboard, Layout, Renderer, Shell, Widget, layout, renderer};
use iced::{Border, Color, Element, Event, Length, Rectangle, Size, mouse, window};

use crate::animate::hover::Hover;
use crate::util::lerp;
use crate::widget::clickable::clickable;
use crate::{border, color, spacing, style};

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
    fn track(self) -> Size {
        match self {
            SwitchSizeVariant::Main => Size::new(48.0, 28.0),
            SwitchSizeVariant::Alt => Size::new(40.0, 24.0),
        }
    }

    fn knob(self) -> f32 {
        match self {
            SwitchSizeVariant::Main => 20.0,
            SwitchSizeVariant::Alt => 16.0,
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
) -> Element<'a, Message>
where
    Message: Clone + 'a,
{
    let track = size.track();
    let knob = SwitchKnob {
        on,
        track,
        knob: size.knob(),
    };

    clickable(knob)
        .background(color::hover_veil())
        .tint(color::border())
        .border(color::border_strong())
        .selected(on)
        .selected_background(color::primary_glass())
        .selected_border(color::primary())
        .selected_fade(style::EMPHASIS)
        .radius(border::ROUNDED_FULL)
        .on_press_maybe(message)
        .into()
}

/// The bespoke part of the switch. Sits inside the clickable chassis at the
/// full track bounds and paints the sliding glass disc. Owns its own slide
/// track so the chassis crossfade and the knob translation flip from the
/// same `on` prop, both on [`style::EMPHASIS`].
struct SwitchKnob {
    on: bool,
    track: Size,
    knob: f32,
}

struct KnobState {
    slide: Hover,
    last_on: bool,
}

impl<Message> Widget<Message, iced::Theme, iced::Renderer> for SwitchKnob {
    fn size(&self) -> Size<Length> {
        Size {
            width: Length::Fixed(self.track.width),
            height: Length::Fixed(self.track.height),
        }
    }

    fn layout(
        &mut self,
        _tree: &mut Tree,
        _renderer: &iced::Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        let bounded = limits
            .width(Length::Fixed(self.track.width))
            .height(Length::Fixed(self.track.height))
            .resolve(
                Length::Fixed(self.track.width),
                Length::Fixed(self.track.height),
                self.track,
            );
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
        let state = tree.state.downcast_ref::<KnobState>();
        let slide = state.slide.current(Instant::now());

        let bounds = layout.bounds();
        let off_x = bounds.x + spacing::PAD_4;
        let on_x = bounds.x + bounds.width - spacing::PAD_4 - self.knob;
        let knob_bounds = Rectangle {
            x: lerp(off_x, on_x, slide),
            y: bounds.y + (bounds.height - self.knob) * 0.5,
            width: self.knob,
            height: self.knob,
        };

        let pill = Border {
            radius: border::ROUNDED_FULL.into(),
            ..Border::default()
        };

        // color::ease lerps the converted alphas of the two tokens, so the
        // mid-slide knob brightness lands on the sRGB midpoint rather than
        // the curve-of-lerped-sRGB the raw alphas would produce.
        let fill = color::ease(color::knob_fill_off(), color::knob_fill_on(), slide);
        renderer.fill_quad(
            Quad {
                bounds: knob_bounds,
                border: pill,
                ..Quad::default()
            },
            fill,
        );

        renderer.fill_quad(
            Quad {
                bounds: knob_bounds,
                border: Border {
                    width: 1.0,
                    color: color::knob_hairline(),
                    ..pill
                },
                ..Quad::default()
            },
            Color::TRANSPARENT,
        );
    }

    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<KnobState>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(KnobState {
            slide: Hover::settled(self.on).with_fade(style::EMPHASIS),
            last_on: self.on,
        })
    }

    fn diff(&self, tree: &mut Tree) {
        let state = tree.state.downcast_mut::<KnobState>();
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
        _layout: Layout<'_>,
        _cursor: mouse::Cursor,
        _renderer: &iced::Renderer,
        _clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
        _viewport: &Rectangle,
    ) {
        let Event::Window(window::Event::RedrawRequested(_)) = event else {
            return;
        };

        let state = tree.state.downcast_ref::<KnobState>();
        if state.slide.animating(Instant::now()) {
            shell.request_redraw();
        }
    }
}

impl<'a, Message: 'a> From<SwitchKnob> for Element<'a, Message> {
    fn from(knob: SwitchKnob) -> Self {
        Element::new(knob)
    }
}
