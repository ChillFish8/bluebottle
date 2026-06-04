//! Shared chassis for the dropdown family. Every styled variant composes on
//! top of it so the trigger affordances and menu motion stay in step.
//!
//! Runs uncontrolled when [`Dropdown::on_toggle`] is left unset. The chassis
//! holds the open state and dismisses on Escape and outside clicks. Wiring a
//! callback puts the caller in control of `expanded`.

use std::time::Instant;

use iced::advanced::renderer::{Quad, Style};
use iced::advanced::widget::{Operation, Tree, tree};
use iced::advanced::{Clipboard, Layout, Renderer, Shell, Widget, layout, overlay};
use iced::keyboard::key::Named;
use iced::keyboard::{self};
use iced::{
    Border,
    Color,
    Element,
    Event,
    Length,
    Padding,
    Point,
    Rectangle,
    Size,
    Transformation,
    Vector,
    alignment,
    mouse,
    touch,
    window,
};

use super::chevron;
use crate::animate::hover::{EPSILON, Hover, PressState};
use crate::{color, style};

const TRIGGER_GAP: f32 = 8.0;
const CHEVRON_SIZE: f32 = 14.0;
const DEFAULT_RADIUS: f32 = 10.0;
const DEFAULT_MENU_RADIUS: f32 = 10.0;
const MENU_OFFSET: f32 = 4.0;
const DEFAULT_TRIGGER_PADDING: Padding = Padding {
    top: 6.0,
    right: 10.0,
    bottom: 6.0,
    left: 12.0,
};
const DEFAULT_MENU_PADDING: Padding = Padding {
    top: 6.0,
    right: 6.0,
    bottom: 6.0,
    left: 6.0,
};

/// Creates a dropdown. The trigger renders `label` next to a chevron and the
/// menu floats below while expanded. Without [`Dropdown::on_toggle`] the
/// chassis self-manages and `expanded` seeds the initial open state.
pub fn dropdown<'a, Message>(
    label: impl Into<Element<'a, Message>>,
    menu: impl Into<Element<'a, Message>>,
    expanded: bool,
) -> Dropdown<'a, Message>
where
    Message: Clone + 'a,
{
    let trigger: Element<'a, Message> = iced::widget::Row::new()
        .push(label.into())
        .push(chevron::chevron(expanded).size(CHEVRON_SIZE))
        .spacing(TRIGGER_GAP)
        .align_y(alignment::Vertical::Center)
        .into();

    Dropdown {
        trigger,
        menu: menu.into(),
        on_toggle: None,
        expanded,
        tint: color::HOVER,
        background: None,
        selected_background: None,
        selected_border: None,
        border: None,
        radius: DEFAULT_RADIUS,
        padding: DEFAULT_TRIGGER_PADDING,
        width: Length::Shrink,
        height: Length::Shrink,
        menu_background: color::SECONDARY,
        menu_border: color::border(),
        menu_radius: DEFAULT_MENU_RADIUS,
        menu_padding: DEFAULT_MENU_PADDING,
        menu_width: Length::Shrink,
    }
}

/// A controlled dropdown built by [`dropdown`].
pub struct Dropdown<'a, Message> {
    trigger: Element<'a, Message>,
    menu: Element<'a, Message>,
    on_toggle: Option<Box<dyn Fn(bool) -> Message + 'a>>,
    expanded: bool,
    tint: Color,
    background: Option<Color>,
    selected_background: Option<Color>,
    selected_border: Option<Color>,
    border: Option<Color>,
    radius: f32,
    padding: Padding,
    width: Length,
    height: Length,
    menu_background: Color,
    menu_border: Color,
    menu_radius: f32,
    menu_padding: Padding,
    menu_width: Length,
}

impl<'a, Message> Dropdown<'a, Message>
where
    Message: Clone + 'a,
{
    /// Forwards open and close events. Wiring this puts the chassis into
    /// controlled mode so the caller owns `expanded`.
    pub fn on_toggle(mut self, f: impl Fn(bool) -> Message + 'a) -> Self {
        self.on_toggle = Some(Box::new(f));
        self
    }

    /// Overrides the hover-tint colour. Defaults to [`color::HOVER`].
    pub fn tint(mut self, color: Color) -> Self {
        self.tint = color;
        self
    }

    /// Paints a solid fill behind the trigger.
    pub fn background(mut self, color: Color) -> Self {
        self.background = Some(color);
        self
    }

    /// The trigger fill shown while the dropdown is open.
    pub fn selected_background(mut self, color: Color) -> Self {
        self.selected_background = Some(color);
        self
    }

    /// The 1 px ring shown around the trigger while the dropdown is open.
    pub fn selected_border(mut self, color: Color) -> Self {
        self.selected_border = Some(color);
        self
    }

    /// A resting 1 px ring around the trigger.
    pub fn border(mut self, color: Color) -> Self {
        self.border = Some(color);
        self
    }

    /// Trigger corner radius.
    pub fn radius(mut self, radius: f32) -> Self {
        self.radius = radius;
        self
    }

    /// Padding around the trigger row.
    pub fn padding(mut self, padding: impl Into<Padding>) -> Self {
        self.padding = padding.into();
        self
    }

    /// Width of the trigger box.
    pub fn width(mut self, width: impl Into<Length>) -> Self {
        self.width = width.into();
        self
    }

    /// Height of the trigger box.
    pub fn height(mut self, height: impl Into<Length>) -> Self {
        self.height = height.into();
        self
    }

    /// Menu surface fill. Defaults to [`color::SECONDARY`].
    pub fn menu_background(mut self, color: Color) -> Self {
        self.menu_background = color;
        self
    }

    /// Menu surface ring. Defaults to [`color::border()`].
    pub fn menu_border(mut self, color: Color) -> Self {
        self.menu_border = color;
        self
    }

    /// Menu corner radius.
    pub fn menu_radius(mut self, radius: f32) -> Self {
        self.menu_radius = radius;
        self
    }

    /// Padding inside the menu surface.
    pub fn menu_padding(mut self, padding: impl Into<Padding>) -> Self {
        self.menu_padding = padding.into();
        self
    }

    /// Width of the menu surface. Defaults to [`Length::Shrink`] so the menu
    /// sizes to its widest row. Pass a fixed width to give a Fill-width child
    /// column something concrete to stretch into so the row chrome paints
    /// edge-to-edge.
    pub fn menu_width(mut self, width: impl Into<Length>) -> Self {
        self.menu_width = width.into();
        self
    }

    fn controlled(&self) -> bool {
        self.on_toggle.is_some()
    }

    fn current_expanded(&self, state: &ChassisState) -> bool {
        if self.controlled() {
            self.expanded
        } else {
            state.expanded
        }
    }
}

#[derive(Default)]
struct ChassisState {
    press: PressState,
    /// Eases between 0 and 1 as `expanded` flips. Drives both the trigger's
    /// selected fill and the overlay menu's roll-down, so the two reads stay
    /// frame-locked without needing two parallel tracks.
    open: Hover,
    /// `expanded` snapshotted at the moment of a press over the trigger so the
    /// matching release toggles against the state we were in when the click
    /// started. Without this snapshot, an external flip between press and
    /// release (e.g. an auto-dismiss timer) inverts the toggle and re-opens
    /// the menu.
    press_expanded: bool,
    /// Source of truth for `expanded` when the chassis runs uncontrolled.
    /// Seeded from `self.expanded` at mount and mutated on press, Escape, and
    /// outside-click. Ignored entirely when `on_toggle` is wired.
    expanded: bool,
}

impl<'a, Message> From<Dropdown<'a, Message>> for Element<'a, Message>
where
    Message: Clone + 'a,
{
    fn from(d: Dropdown<'a, Message>) -> Self {
        Element::new(d)
    }
}

impl<'a, Message> Widget<Message, iced::Theme, iced::Renderer> for Dropdown<'a, Message>
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
            self.trigger
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
        let state = tree.state.downcast_ref::<ChassisState>();
        let now = Instant::now();
        let bounds = layout.bounds();

        let hover_factor = state.press.hover.current(now);
        let selected_factor = state.open.current(now);
        let glass_factor = if self.selected_background.is_some() {
            hover_factor * (1.0 - selected_factor)
        } else {
            hover_factor
        };

        let pill = Border {
            radius: self.radius.into(),
            ..Border::default()
        };

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

        let content_style = Style {
            text_color: style.text_color,
        };

        let trigger_layout = layout.children().next().expect("dropdown trigger");
        self.trigger.as_widget().draw(
            tree.children.first().expect("dropdown trigger tree"),
            renderer,
            theme,
            &content_style,
            trigger_layout,
            cursor,
            viewport,
        );
    }

    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<ChassisState>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(ChassisState {
            press: PressState::default(),
            open: Hover::settled(self.expanded),
            press_expanded: self.expanded,
            expanded: self.expanded,
        })
    }

    fn children(&self) -> Vec<Tree> {
        vec![Tree::new(&self.trigger), Tree::new(&self.menu)]
    }

    fn diff(&self, tree: &mut Tree) {
        tree.diff_children(&[&self.trigger, &self.menu]);

        let current = {
            let state = tree.state.downcast_mut::<ChassisState>();
            let value = self.current_expanded(state);
            state.open.flip(value, Instant::now());
            value
        };

        // The chevron's `open` was baked into the trigger Element at builder
        // time. In controlled mode the caller threads the current value
        // through, so the chevron tracks. In uncontrolled mode we override
        // the freshly-applied flip with the live state value here. The
        // trigger is a Row of [label, chevron] so the chevron tree sits at
        // children[1] under the trigger tree.
        if !self.controlled()
            && let Some(trigger_tree) = tree.children.first_mut()
            && let Some(chevron_tree) = trigger_tree.children.get_mut(1)
        {
            chevron::flip_state(chevron_tree, current);
        }
    }

    fn operate(
        &mut self,
        tree: &mut Tree,
        layout: Layout<'_>,
        renderer: &iced::Renderer,
        operation: &mut dyn Operation,
    ) {
        let trigger_layout = layout.children().next().expect("dropdown trigger");
        self.trigger.as_widget_mut().operate(
            &mut tree.children[0],
            trigger_layout,
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
        let trigger_layout = layout.children().next().expect("dropdown trigger");

        self.trigger.as_widget_mut().update(
            &mut tree.children[0],
            event,
            trigger_layout,
            cursor,
            renderer,
            clipboard,
            shell,
            viewport,
        );

        let now = Instant::now();
        let bounds = layout.bounds();
        let over = cursor.is_over(bounds);
        let state = tree.state.downcast_mut::<ChassisState>();

        match event {
            // Release dispatch (matching the design system's button convention)
            // with a press-time snapshot of `expanded`. iced masks the main
            // tree's cursor while the menu overlay is alive, so the trigger
            // only ever sees press-release pairs in the closed-to-open
            // direction, but the snapshot still guards against `expanded`
            // flipping mid-cycle through some other path (auto-dismiss
            // timer, programmatic close, etc.) and pinning the toggle to
            // the value the user was acting on.
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) => {
                if !shell.is_event_captured() {
                    let landed = state.press.press(over);
                    if landed {
                        state.press_expanded = if self.on_toggle.is_some() {
                            self.expanded
                        } else {
                            state.expanded
                        };
                    }
                }
            },

            Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)) => {
                let dispatch = state.press.release(over);
                if dispatch && !shell.is_event_captured() {
                    let next = !state.press_expanded;
                    if let Some(on_toggle) = &self.on_toggle {
                        shell.publish(on_toggle(next));
                    } else {
                        state.expanded = next;
                        shell.request_redraw();
                    }
                    shell.capture_event();
                }
            },

            _ => {
                if state.press.reconcile(over, now) {
                    shell.request_redraw();
                }
                if let Event::Window(window::Event::RedrawRequested(_)) = event
                    && (state.press.animating(now) || state.open.animating(now))
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
        let trigger_layout = layout.children().next().expect("dropdown trigger");
        let inner = self.trigger.as_widget().mouse_interaction(
            &tree.children[0],
            trigger_layout,
            cursor,
            viewport,
            renderer,
        );
        if !matches!(inner, mouse::Interaction::None | mouse::Interaction::Idle) {
            return inner;
        }

        if cursor.is_over(layout.bounds()) {
            return mouse::Interaction::Pointer;
        }

        mouse::Interaction::None
    }

    fn overlay<'b>(
        &'b mut self,
        tree: &'b mut Tree,
        layout: Layout<'b>,
        renderer: &iced::Renderer,
        viewport: &Rectangle,
        translation: Vector,
    ) -> Option<overlay::Element<'b, Message, iced::Theme, iced::Renderer>> {
        // Split-borrow `state` from `children` so the uncontrolled overlay
        // can hold a mutable reference into `state.expanded` for the lifetime
        // of the overlay element.
        let Tree {
            state, children, ..
        } = tree;
        let state = state.downcast_mut::<ChassisState>();
        let factor = state.open.current(Instant::now());
        let controlled = self.on_toggle.is_some();
        let current_expanded = if controlled {
            self.expanded
        } else {
            state.expanded
        };
        let alive = current_expanded || factor > EPSILON;

        let trigger_layout = layout.children().next().expect("dropdown trigger");
        let local_bounds = layout.bounds();
        let position = layout.position() + translation;

        // Trigger and menu trees are disjoint slots; split lets both overlay
        // paths borrow the children mutably for the same widget call.
        let (trigger_children, menu_children) = children.split_at_mut(1);
        let trigger_tree = &mut trigger_children[0];
        let menu_tree = &mut menu_children[0];

        // Always give the trigger a chance to publish its own overlay (tooltip,
        // nested popover, etc.). Without this, wrapping any overlay-producing
        // widget in the trigger silently drops its overlay both while open and
        // while closed.
        let trigger_overlay = self.trigger.as_widget_mut().overlay(
            trigger_tree,
            trigger_layout,
            renderer,
            viewport,
            translation,
        );

        let menu_overlay = if alive {
            // Stamp trigger_bounds at the translated origin so the dismiss-guard
            // and the menu anchor share a single screen-space coordinate system,
            // independent of any later layout/draw bounds.
            let trigger_bounds = Rectangle {
                x: position.x,
                y: position.y,
                width: local_bounds.width,
                height: local_bounds.height,
            };
            let uncontrolled_expanded = if controlled {
                None
            } else {
                Some(&mut state.expanded)
            };
            Some(overlay::Element::new(Box::new(DropdownOverlay {
                menu_tree,
                menu: &mut self.menu,
                on_toggle: self.on_toggle.as_deref(),
                uncontrolled_expanded,
                expanded: current_expanded,
                factor,
                trigger_bounds,
                position,
                viewport: *viewport,
                background: self.menu_background,
                border: self.menu_border,
                radius: self.menu_radius,
                padding: self.menu_padding,
                offset: MENU_OFFSET,
                width: self.menu_width,
            })))
        } else {
            None
        };

        match (trigger_overlay, menu_overlay) {
            (None, None) => None,
            (Some(child), None) | (None, Some(child)) => Some(child),
            (Some(trigger), Some(menu)) => {
                Some(overlay::Group::with_children(vec![trigger, menu]).overlay())
            },
        }
    }
}

impl<Message> DropdownOverlay<'_, '_, Message> {
    fn viewport_size(&self) -> Size {
        Size::new(self.viewport.width, self.viewport.height)
    }

    /// Vertical clearance available below and above the trigger, taking the
    /// menu offset into account. Both values are clamped to zero.
    fn vertical_space(&self) -> (f32, f32) {
        let viewport = self.viewport_size();
        let below = (viewport.height
            - (self.position.y + self.trigger_bounds.height + self.offset))
            .max(0.0);
        let above = (self.position.y - self.offset).max(0.0);
        (below, above)
    }

    /// Top-left the menu should land at given its size. Same algorithm as
    /// [`Self::layout`] so the rendering path can re-derive the anchor against
    /// the trigger's just-captured position. iced reuses the overlay layout
    /// from the previous update pass, so without this the menu renders one
    /// scroll-step behind the trigger.
    ///
    /// Prefers below when the menu fits there. Falls back to above when below
    /// does not fit so the menu never overlaps the trigger. The layout pass
    /// caps the menu's height to the larger of the two clearances, so one of
    /// the two sides is guaranteed to fit.
    fn menu_anchor(&self, menu_size: Size) -> Point {
        let viewport = self.viewport_size();
        let (space_below, space_above) = self.vertical_space();

        let below_y = self.position.y + self.trigger_bounds.height + self.offset;
        let above_y = self.position.y - menu_size.height - self.offset;

        // Prefer the side that fits. When neither fits, anchor against the
        // side with more clearance so the menu grows away from the trigger
        // rather than across it. The menu may overflow the viewport edge but
        // never overlaps the trigger.
        let y = if menu_size.height <= space_below {
            below_y
        } else if menu_size.height <= space_above {
            above_y
        } else if space_below >= space_above {
            below_y
        } else {
            above_y
        };

        let mut pos = Point::new(self.position.x, y);

        if pos.x + menu_size.width > viewport.width {
            pos.x = (viewport.width - menu_size.width).max(0.0);
        }
        if pos.x < 0.0 {
            pos.x = 0.0;
        }

        pos
    }

    /// Returns `(fresh_bounds, shift)` for compensating the gap between
    /// iced's stored overlay layout and the trigger's current position. The
    /// shift is zero when nothing has moved between update and draw.
    fn lag_compensation(&self, stored: Rectangle) -> (Rectangle, Vector) {
        let anchor = self.menu_anchor(stored.size());

        let fresh = Rectangle {
            x: anchor.x,
            y: anchor.y,
            width: stored.width,
            height: stored.height,
        };
        let shift = Vector::new(anchor.x - stored.x, anchor.y - stored.y);

        (fresh, shift)
    }

    /// Same anchor math as [`Self::lag_compensation`] but skips building the
    /// fresh rectangle when only the shift is needed.
    fn lag_shift(&self, stored: Rectangle) -> Vector {
        let anchor = self.menu_anchor(stored.size());
        Vector::new(anchor.x - stored.x, anchor.y - stored.y)
    }
}

struct DropdownOverlay<'a, 'b, Message> {
    menu_tree: &'b mut Tree,
    menu: &'b mut Element<'a, Message>,
    on_toggle: Option<&'b (dyn Fn(bool) -> Message + 'a)>,
    /// Direct write path into `ChassisState.expanded` for uncontrolled mode.
    /// `Some` here is the marker for self-managing dismissal. `None` means the
    /// chassis is controlled and `on_toggle` carries the close.
    uncontrolled_expanded: Option<&'b mut bool>,
    expanded: bool,
    factor: f32,
    trigger_bounds: Rectangle,
    position: Point,
    viewport: Rectangle,
    background: Color,
    border: Color,
    radius: f32,
    padding: Padding,
    offset: f32,
    width: Length,
}

impl<Message> overlay::Overlay<Message, iced::Theme, iced::Renderer>
    for DropdownOverlay<'_, '_, Message>
where
    Message: Clone,
{
    fn layout(&mut self, renderer: &iced::Renderer, bounds: Size) -> layout::Node {
        // Cap the menu's max height to whichever side of the trigger has more
        // clearance. Without this the menu can size itself taller than either
        // side of the trigger and the anchor step has nowhere to put it, so it
        // ends up overlapping the trigger.
        //
        // When both clearances collapse to zero (trigger fully off-screen, or
        // overlay alive while the trigger has scrolled away) fall back to the
        // full viewport so the menu still lays out at its natural size. The
        // anchor step lets it render against whichever viewport edge fits.
        let (space_below, space_above) = self.vertical_space();
        let preferred = space_below.max(space_above);
        let max_height = if preferred > 0.0 {
            preferred.min(bounds.height)
        } else {
            bounds.height
        };

        let menu_bounds = Size::new(bounds.width, max_height);
        let limits = layout::Limits::new(Size::ZERO, menu_bounds)
            .width(self.width)
            .height(Length::Shrink);

        let node =
            layout::padded(&limits, self.width, Length::Shrink, self.padding, |inner| {
                self.menu
                    .as_widget_mut()
                    .layout(self.menu_tree, renderer, inner)
            });

        let menu_size = node.bounds().size();

        node.move_to(self.menu_anchor(menu_size))
    }

    fn draw(
        &self,
        renderer: &mut iced::Renderer,
        theme: &iced::Theme,
        style: &iced::advanced::renderer::Style,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
    ) {
        let stored = layout.bounds();
        let factor = self.factor;

        if factor <= EPSILON {
            return;
        }

        // iced rebuilds our overlay struct on every draw so `self.position`
        // is current, but reuses the layout captured during the previous
        // update pass. Re-derive the anchor from the live position and
        // translate the renderer by the delta so the menu lands where the
        // trigger is now rather than one frame back.
        let shift = self.lag_shift(stored);

        let pill = Border {
            radius: self.radius.into(),
            ..Border::default()
        };

        // Scale the chrome (surface fill and 1 px ring) by factor so the
        // whole menu fades together rather than the rectangle popping in
        // beneath a fading content layer.
        let background = Color {
            a: self.background.a * factor,
            ..self.background
        };
        let border = Color {
            a: self.border.a * factor,
            ..self.border
        };

        renderer.with_translation(shift, |renderer| {
            renderer.fill_quad(
                Quad {
                    bounds: stored,
                    border: pill,
                    shadow: style::scale_shadow(style::ELEVATION_RESTING, factor),
                    ..Quad::default()
                },
                Color::TRANSPARENT,
            );

            renderer.with_layer(stored, |renderer| {
                renderer.fill_quad(
                    Quad {
                        bounds: stored,
                        border: pill,
                        ..Quad::default()
                    },
                    background,
                );
                renderer.fill_quad(
                    Quad {
                        bounds: stored,
                        border: Border {
                            radius: self.radius.into(),
                            width: 1.0,
                            color: border,
                        },
                        ..Quad::default()
                    },
                    Color::TRANSPARENT,
                );

                let content_layout =
                    layout.children().next().expect("dropdown menu child");
                self.menu.as_widget().draw(
                    self.menu_tree,
                    renderer,
                    theme,
                    style,
                    content_layout,
                    cursor * Transformation::translate(-shift.x, -shift.y),
                    &stored,
                );

                // Fade the whole menu in and out by tinting the content
                // layer with the surface colour at (1 - factor) opacity.
                // The alpha is taken straight from `1 - factor` rather than
                // scaled by `self.background.a`, so a translucent menu
                // surface still gets a fully-opaque veil at factor = 0 and
                // the content underneath cannot peek through.
                let veil_alpha = (1.0 - factor).clamp(0.0, 1.0);
                if veil_alpha > EPSILON {
                    renderer.fill_quad(
                        Quad {
                            bounds: stored,
                            border: pill,
                            ..Quad::default()
                        },
                        Color {
                            a: veil_alpha,
                            ..self.background
                        },
                    );
                }
            });
        });
    }

    fn operate(
        &mut self,
        layout: Layout<'_>,
        renderer: &iced::Renderer,
        operation: &mut dyn Operation,
    ) {
        let content_layout = layout.children().next().expect("dropdown menu child");
        self.menu.as_widget_mut().operate(
            self.menu_tree,
            content_layout,
            renderer,
            operation,
        );
    }

    fn update(
        &mut self,
        event: &Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &iced::Renderer,
        clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
    ) {
        if !self.expanded {
            return;
        }

        let stored = layout.bounds();
        let content_layout = layout.children().next().expect("dropdown menu child");
        let (fresh, shift) = self.lag_compensation(stored);

        // The menu fades rather than rolls, but rows still should not react
        // to clicks while the fade is in flight. Mask the cursor while the
        // menu is mid-animation so accidental presses during the open or
        // close band do not register on whatever happens to sit underneath
        // the cursor at the moment.
        let translated = cursor * Transformation::translate(-shift.x, -shift.y);
        let menu_cursor = if self.factor < 1.0 - EPSILON {
            mouse::Cursor::Unavailable
        } else {
            translated
        };

        self.menu.as_widget_mut().update(
            self.menu_tree,
            event,
            content_layout,
            menu_cursor,
            renderer,
            clipboard,
            shell,
            &stored,
        );

        if shell.is_event_captured() {
            return;
        }

        let dismiss = match event {
            Event::Keyboard(keyboard::Event::KeyPressed { key, .. })
                if key == &keyboard::Key::Named(Named::Escape) =>
            {
                true
            },

            // Left-press anywhere outside the menu dismisses, trigger
            // included. iced masks the main tree's cursor to Unavailable
            // while any overlay claims a region, so the chassis trigger
            // never sees the closing press through its own update path,
            // and routing the close through the dismiss-guard here keeps
            // trigger-to-close working. Capturing prevents the press from
            // also activating whatever sits below.
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left))
                if !cursor.is_over(fresh) =>
            {
                true
            },

            // Right-press still treats the trigger as inside so a future
            // context-menu on the trigger is not silently dismissed.
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Right))
                if !cursor.is_over(fresh) && !cursor.is_over(self.trigger_bounds) =>
            {
                true
            },

            // Touch follows the left-mouse rule. iced masks touch positions
            // the same way it masks the cursor, so trigger-tap-to-close
            // also needs the dismiss-guard path.
            Event::Touch(touch::Event::FingerPressed { position, .. })
                if !fresh.contains(*position) =>
            {
                true
            },

            _ => false,
        };

        if !dismiss {
            return;
        }

        if let Some(on_toggle) = self.on_toggle {
            shell.publish(on_toggle(false));
            shell.capture_event();
        } else if let Some(expanded) = self.uncontrolled_expanded.as_deref_mut() {
            *expanded = false;
            shell.request_redraw();
            shell.capture_event();
        }
    }

    fn mouse_interaction(
        &self,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &iced::Renderer,
    ) -> mouse::Interaction {
        if !self.expanded {
            return mouse::Interaction::None;
        }
        let stored = layout.bounds();
        let shift = self.lag_shift(stored);
        let translated = cursor * Transformation::translate(-shift.x, -shift.y);
        let menu_cursor = if self.factor < 1.0 - EPSILON {
            mouse::Cursor::Unavailable
        } else {
            translated
        };
        let content_layout = layout.children().next().expect("dropdown menu child");
        self.menu.as_widget().mouse_interaction(
            self.menu_tree,
            content_layout,
            menu_cursor,
            &self.viewport,
            renderer,
        )
    }
}
