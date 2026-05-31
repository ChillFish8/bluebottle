//! The core dropdown widget. Every styled dropdown in the design system
//! composes around this chassis so the trigger affordances and menu motion stay
//! in step with the rest of the system.
//!
//! The chassis owns a trigger row of `[label, chevron]` and a menu that floats
//! below it. The trigger paints the same hover glass, selected fill, and 1 px
//! ring vocabulary as [`crate::widget::clickable::Clickable`] so a dropdown
//! reads as a button at rest. The menu eases in with a 100 ms grow and fade on
//! the design system's `Hover` budget, anchored below the trigger, sized to
//! its content, and clamped inside the viewport.
//!
//! Controlled by the caller. `expanded` is held in the caller's state and the
//! widget dispatches [`Dropdown::on_toggle`] on trigger press, on Escape, and
//! on a click outside the menu while it is open. Without `on_toggle` the
//! widget is inert.

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
const DEFAULT_MENU_OFFSET: f32 = 4.0;
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

/// Creates a controlled dropdown. The trigger renders the `label` and a
/// chevron, the menu floats below it while `expanded` is true. Inert until
/// [`Dropdown::on_toggle`] is set.
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
        resting_color: None,
        background: None,
        selected_background: None,
        selected_border: None,
        selected_color: None,
        border: None,
        hover_border: None,
        radius: DEFAULT_RADIUS,
        padding: DEFAULT_TRIGGER_PADDING,
        width: Length::Shrink,
        height: Length::Shrink,
        menu_background: color::SECONDARY,
        menu_border: color::border(),
        menu_radius: DEFAULT_MENU_RADIUS,
        menu_padding: DEFAULT_MENU_PADDING,
        menu_offset: DEFAULT_MENU_OFFSET,
    }
}

/// A controlled dropdown built by [`dropdown`].
pub struct Dropdown<'a, Message> {
    trigger: Element<'a, Message>,
    menu: Element<'a, Message>,
    on_toggle: Option<Box<dyn Fn(bool) -> Message + 'a>>,
    expanded: bool,
    tint: Color,
    resting_color: Option<Color>,
    background: Option<Color>,
    selected_background: Option<Color>,
    selected_border: Option<Color>,
    selected_color: Option<Color>,
    border: Option<Color>,
    hover_border: Option<Color>,
    radius: f32,
    padding: Padding,
    width: Length,
    height: Length,
    menu_background: Color,
    menu_border: Color,
    menu_radius: f32,
    menu_padding: Padding,
    menu_offset: f32,
}

impl<'a, Message> Dropdown<'a, Message>
where
    Message: Clone + 'a,
{
    /// Sets the toggle callback. Fires `true` when the trigger opens the menu
    /// and `false` on close, dismiss, or Escape. Required to make the widget
    /// interactive.
    pub fn on_toggle(mut self, f: impl Fn(bool) -> Message + 'a) -> Self {
        self.on_toggle = Some(Box::new(f));
        self
    }

    /// Overrides the hover-tint colour. Defaults to [`color::HOVER`].
    pub fn tint(mut self, color: Color) -> Self {
        self.tint = color;
        self
    }

    /// Overrides the resting label and chevron colour.
    pub fn resting_color(mut self, color: Color) -> Self {
        self.resting_color = Some(color);
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

    /// The label and chevron colour while the dropdown is open. Eases from
    /// the resting colour.
    pub fn selected_color(mut self, color: Color) -> Self {
        self.selected_color = Some(color);
        self
    }

    /// A resting 1 px ring around the trigger.
    pub fn border(mut self, color: Color) -> Self {
        self.border = Some(color);
        self
    }

    /// A 1 px ring that fades in with the hover tint.
    pub fn hover_border(mut self, color: Color) -> Self {
        self.hover_border = Some(color);
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

    /// Vertical gap between the trigger and the menu, in logical pixels.
    pub fn menu_offset(mut self, offset: f32) -> Self {
        self.menu_offset = offset;
        self
    }

    fn interactive(&self) -> bool {
        self.on_toggle.is_some()
    }
}

#[derive(Default)]
struct ChassisState {
    press: PressState,
    /// Eases between 0 and 1 as `expanded` flips. Drives both the trigger's
    /// selected fill and the overlay menu's roll-down, so the two reads stay
    /// frame-locked without needing two parallel tracks.
    open: Hover,
    /// `self.expanded` snapshotted at the moment of a press over the trigger
    /// so the matching release dispatches against the state we were in when
    /// the click started. Without this snapshot, an external flip between
    /// press and release (e.g. an auto-dismiss timer) inverts the toggle and
    /// re-opens the menu.
    press_expanded: bool,
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

        let hover_factor = if self.interactive() {
            state.press.hover.current(now)
        } else {
            0.0
        };
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

        let resting = self.resting_color.unwrap_or(style.text_color);
        let text_color = match self.selected_color {
            Some(on) => color::ease(resting, on, selected_factor),
            None => resting,
        };
        let content_style = Style { text_color };

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
        })
    }

    fn children(&self) -> Vec<Tree> {
        vec![Tree::new(&self.trigger), Tree::new(&self.menu)]
    }

    fn diff(&self, tree: &mut Tree) {
        tree.diff_children(&[&self.trigger, &self.menu]);

        let state = tree.state.downcast_mut::<ChassisState>();
        state.open.flip(self.expanded, Instant::now());
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

        if !self.interactive() {
            return;
        }

        let now = Instant::now();
        let bounds = layout.bounds();
        let over = cursor.is_over(bounds);
        let state = tree.state.downcast_mut::<ChassisState>();

        match event {
            // Release dispatch (matching the design system's button convention)
            // with a press-time snapshot of `expanded`. The snapshot is what
            // closes the re-open race: if `expanded` flips between press and
            // release (auto-dismiss, an outside-click handler, etc.), the
            // release still toggles relative to the value the user was acting
            // on, not the new one.
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) => {
                if !shell.is_event_captured() {
                    let landed = state.press.press(over);
                    if landed {
                        state.press_expanded = self.expanded;
                    }
                }
            },

            Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)) => {
                let dispatch = state.press.release(over);
                if dispatch
                    && !shell.is_event_captured()
                    && let Some(on_toggle) = &self.on_toggle
                {
                    shell.publish(on_toggle(!state.press_expanded));
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

        if self.interactive() && cursor.is_over(layout.bounds()) {
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
        let factor = tree
            .state
            .downcast_ref::<ChassisState>()
            .open
            .current(Instant::now());
        let alive = self.expanded || factor > EPSILON;

        let trigger_layout = layout.children().next().expect("dropdown trigger");
        let local_bounds = layout.bounds();
        let position = layout.position() + translation;

        // Trigger and menu trees are disjoint slots; split lets both overlay
        // paths borrow the children mutably for the same widget call.
        let (trigger_children, menu_children) = tree.children.split_at_mut(1);
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
            Some(overlay::Element::new(Box::new(DropdownOverlay {
                menu_tree,
                menu: &mut self.menu,
                on_toggle: self.on_toggle.as_deref(),
                expanded: self.expanded,
                factor,
                trigger_bounds,
                position,
                viewport: *viewport,
                background: self.menu_background,
                border: self.menu_border,
                radius: self.menu_radius,
                padding: self.menu_padding,
                offset: self.menu_offset,
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

struct DropdownOverlay<'a, 'b, Message> {
    menu_tree: &'b mut Tree,
    menu: &'b mut Element<'a, Message>,
    on_toggle: Option<&'b (dyn Fn(bool) -> Message + 'a)>,
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
}

impl<Message> overlay::Overlay<Message, iced::Theme, iced::Renderer>
    for DropdownOverlay<'_, '_, Message>
where
    Message: Clone,
{
    fn layout(&mut self, renderer: &iced::Renderer, bounds: Size) -> layout::Node {
        let limits = layout::Limits::new(Size::ZERO, bounds)
            .width(Length::Shrink)
            .height(Length::Shrink);

        let node = layout::padded(
            &limits,
            Length::Shrink,
            Length::Shrink,
            self.padding,
            |inner| {
                self.menu
                    .as_widget_mut()
                    .layout(self.menu_tree, renderer, inner)
            },
        );

        let menu_size = node.bounds().size();
        let mut pos = Point::new(
            self.position.x,
            self.position.y + self.trigger_bounds.height + self.offset,
        );

        if pos.x + menu_size.width > bounds.width {
            pos.x = (bounds.width - menu_size.width).max(0.0);
        }
        if pos.x < 0.0 {
            pos.x = 0.0;
        }
        if pos.y + menu_size.height > bounds.height {
            let above = self.position.y - menu_size.height - self.offset;
            if above >= 0.0 {
                pos.y = above;
            } else {
                pos.y = (bounds.height - menu_size.height).max(0.0);
            }
        }

        node.move_to(pos)
    }

    fn draw(
        &self,
        renderer: &mut iced::Renderer,
        theme: &iced::Theme,
        style: &iced::advanced::renderer::Style,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
    ) {
        let bounds = layout.bounds();
        let factor = self.factor;
        if factor <= EPSILON {
            return;
        }

        // Roll the menu down by revealing a top strip of its full layout that
        // grows from zero to full height with the open factor. Content is laid
        // out at full size and clipped, so the menu drops on open and rolls back
        // on close without scaling its contents.
        let revealed_height = (bounds.height * factor).max(0.0);
        let revealed = Rectangle {
            x: bounds.x,
            y: bounds.y,
            width: bounds.width,
            height: revealed_height,
        };
        let pill = Border {
            radius: self.radius.into(),
            ..Border::default()
        };

        // The shadow sits below the visible strip so the elevation grows with
        // the menu. Drawn outside the clip so the soft edge extends past the
        // revealed band.
        renderer.fill_quad(
            Quad {
                bounds: revealed,
                border: pill,
                shadow: style::scale_shadow(style::ELEVATION_RESTING, factor),
                ..Quad::default()
            },
            Color::TRANSPARENT,
        );

        renderer.with_layer(revealed, |renderer| {
            renderer.fill_quad(
                Quad {
                    bounds,
                    border: pill,
                    ..Quad::default()
                },
                self.background,
            );
            renderer.fill_quad(
                Quad {
                    bounds,
                    border: Border {
                        radius: self.radius.into(),
                        width: 1.0,
                        color: self.border,
                    },
                    ..Quad::default()
                },
                Color::TRANSPARENT,
            );

            let content_layout = layout.children().next().expect("dropdown menu child");
            self.menu.as_widget().draw(
                self.menu_tree,
                renderer,
                theme,
                style,
                content_layout,
                cursor,
                &bounds,
            );
        });
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

        let bounds = layout.bounds();
        let content_layout = layout.children().next().expect("dropdown menu child");

        // The visible band is `revealed`. While the menu is rolling open, the
        // cursor may sit over a clipped (invisible) item — masking the cursor
        // there keeps the menu's children from registering hover, press, or
        // pointer cursor for content the user cannot see yet.
        let menu_cursor = mask_cursor_to_revealed(cursor, bounds, self.factor);

        self.menu.as_widget_mut().update(
            self.menu_tree,
            event,
            content_layout,
            menu_cursor,
            renderer,
            clipboard,
            shell,
            &bounds,
        );

        if shell.is_event_captured() {
            return;
        }

        let Some(on_toggle) = self.on_toggle else {
            return;
        };

        match event {
            Event::Keyboard(keyboard::Event::KeyPressed { key, .. })
                if key == &keyboard::Key::Named(Named::Escape) =>
            {
                shell.publish(on_toggle(false));
                shell.capture_event();
            },

            // Capture the dismiss so the same press does not also activate a
            // widget underneath the open menu. The trigger path captures on
            // toggle for the same reason; this keeps the two consistent.
            Event::Mouse(mouse::Event::ButtonPressed(
                mouse::Button::Left | mouse::Button::Right,
            )) if !cursor.is_over(bounds) && !cursor.is_over(self.trigger_bounds) => {
                shell.publish(on_toggle(false));
                shell.capture_event();
            },

            Event::Touch(touch::Event::FingerPressed { position, .. })
                if !bounds.contains(*position)
                    && !self.trigger_bounds.contains(*position) =>
            {
                shell.publish(on_toggle(false));
                shell.capture_event();
            },

            _ => {},
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
        let bounds = layout.bounds();
        let menu_cursor = mask_cursor_to_revealed(cursor, bounds, self.factor);
        let content_layout = layout.children().next().expect("dropdown menu child");
        self.menu.as_widget().mouse_interaction(
            self.menu_tree,
            content_layout,
            menu_cursor,
            &self.viewport,
            renderer,
        )
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
}

/// Returns the cursor as-is when it sits inside the menu's currently-revealed
/// band, and [`mouse::Cursor::Unavailable`] when it sits over a clipped (and
/// therefore invisible) part of the menu. The cursor passes through unchanged
/// when it is outside the menu's full bounds, so the dismiss-guard above
/// still sees real positions.
fn mask_cursor_to_revealed(
    cursor: mouse::Cursor,
    bounds: Rectangle,
    factor: f32,
) -> mouse::Cursor {
    let Some(position) = cursor.position() else {
        return cursor;
    };
    if !bounds.contains(position) {
        return cursor;
    }
    let revealed = Rectangle {
        height: bounds.height * factor.clamp(0.0, 1.0),
        ..bounds
    };
    if revealed.contains(position) {
        cursor
    } else {
        mouse::Cursor::Unavailable
    }
}
