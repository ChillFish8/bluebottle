//! A header and a body laid out vertically. The header pins to the top of
//! the enclosing scrollable once the user scrolls past it, and returns to
//! its natural place when the user scrolls back above. A 1px structural
//! border fades in beneath the pinned header so the surface separates
//! from the scrolling rows.
//!
//! Owning the body inside the widget is what keeps the wrapper visible to
//! a parent column. iced's column culls children whose layout does not
//! intersect the viewport, which would skip the header's draw the moment
//! it scrolled past. Bundling header and body together keeps the bounds
//! spanning both, so the wrapper stays drawn while any part of the
//! section is visible.

use iced::advanced::renderer::Quad;
use iced::advanced::widget::{Operation, Tree};
use iced::advanced::{
    Clipboard,
    Layout,
    Renderer,
    Shell,
    Widget,
    layout,
    overlay,
    renderer,
};
use iced::{
    Border,
    Element,
    Event,
    Length,
    Point,
    Rectangle,
    Renderer as IcedRenderer,
    Size,
    Theme,
    Transformation,
    Vector,
    mouse,
};

use crate::color;

const DIVIDER_FADE_PX: f32 = 8.0;

/// Builds a sticky section. The header pins to the top of the enclosing
/// scrollable once the user scrolls past it. The body sits beneath the
/// header and scrolls normally.
pub fn sticky<'a, Message>(
    header: impl Into<Element<'a, Message>>,
    body: impl Into<Element<'a, Message>>,
) -> Sticky<'a, Message> {
    Sticky {
        header: header.into(),
        body: body.into(),
        divider: true,
        spacing: 0.0,
        top: 0.0,
    }
}

/// A sticky section, built by [`sticky`].
pub struct Sticky<'a, Message> {
    header: Element<'a, Message>,
    body: Element<'a, Message>,
    divider: bool,
    spacing: f32,
    top: f32,
}

impl<'a, Message> Sticky<'a, Message> {
    /// Toggles the 1px structural border that fades in once content has
    /// scrolled beneath the sticky header. Default is on.
    pub fn divider(mut self, show: bool) -> Self {
        self.divider = show;
        self
    }

    /// Sets the vertical gap between the header and the body. Defaults to
    /// no gap.
    pub fn spacing(mut self, gap: f32) -> Self {
        self.spacing = gap;
        self
    }

    /// Sets the inset between the top of the scrollable and the pinned
    /// header. Matches CSS `position: sticky` with `top: N`. The header
    /// pins at this distance below the viewport edge once its natural
    /// position would scroll past the inset.
    pub fn top(mut self, inset: f32) -> Self {
        self.top = inset;
        self
    }
}

impl<'a, Message> From<Sticky<'a, Message>> for Element<'a, Message>
where
    Message: 'a,
{
    fn from(s: Sticky<'a, Message>) -> Self {
        Element::new(s)
    }
}

fn stuck_for(header: Layout<'_>, viewport: &Rectangle, top: f32) -> f32 {
    ((viewport.y + top) - header.bounds().y).max(0.0)
}

fn pinned_bounds(
    header: Layout<'_>,
    viewport: &Rectangle,
    top: f32,
    stuck: f32,
) -> Rectangle {
    if stuck > 0.0 {
        Rectangle::new(
            Point::new(viewport.x, viewport.y + top),
            Size::new(viewport.width, header.bounds().height),
        )
    } else {
        header.bounds()
    }
}

impl<'a, Message> Widget<Message, Theme, IcedRenderer> for Sticky<'a, Message>
where
    Message: 'a,
{
    fn size(&self) -> Size<Length> {
        Size {
            width: Length::Fill,
            height: Length::Shrink,
        }
    }

    fn layout(
        &mut self,
        tree: &mut Tree,
        renderer: &IcedRenderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        let outer = limits.width(Length::Fill).height(Length::Shrink);

        let header_node = self.header.as_widget_mut().layout(
            &mut tree.children[0],
            renderer,
            &outer.loose(),
        );
        let header_size = header_node.size();

        // Reserve the top inset in the body's layout. Without this, when
        // the navbar is pinned the body's first `top` pixels would render
        // underneath it. Matches CSS sticky composed with a parent
        // padding-top of the same magnitude.
        let body_y = header_size.height + self.spacing + self.top;
        let body_limits = outer.shrink(Size::new(0.0, body_y));
        let body_node = self.body.as_widget_mut().layout(
            &mut tree.children[1],
            renderer,
            &body_limits.loose(),
        );
        let body_size = body_node.size();

        let width = header_size.width.max(body_size.width);
        let height = body_y + body_size.height;

        layout::Node::with_children(
            Size::new(width, height),
            vec![
                header_node.move_to(Point::new(0.0, 0.0)),
                body_node.move_to(Point::new(0.0, body_y)),
            ],
        )
    }

    fn draw(
        &self,
        tree: &Tree,
        renderer: &mut IcedRenderer,
        theme: &Theme,
        style: &renderer::Style,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
    ) {
        let mut children = layout.children();
        let header_layout = children.next().unwrap();
        let body_layout = children.next().unwrap();

        self.body.as_widget().draw(
            &tree.children[1],
            renderer,
            theme,
            style,
            body_layout,
            cursor,
            viewport,
        );

        let stuck = stuck_for(header_layout, viewport, self.top);
        let header_bounds = header_layout.bounds();

        if stuck <= 0.0 {
            self.header.as_widget().draw(
                &tree.children[0],
                renderer,
                theme,
                style,
                header_layout,
                cursor,
                viewport,
            );
            return;
        }

        // The header renders translated by stuck in y, so its visible top
        // lands at viewport.y + top. The cursor and viewport handed to the
        // child are remapped by the inverse so it hit-tests against its
        // natural layout.
        let shift = Transformation::translate(0.0, -stuck);
        let child_cursor = cursor * shift;
        let child_viewport = *viewport * shift;

        renderer.with_layer(*viewport, |renderer| {
            renderer.with_translation(Vector::new(0.0, stuck), |renderer| {
                self.header.as_widget().draw(
                    &tree.children[0],
                    renderer,
                    theme,
                    style,
                    header_layout,
                    child_cursor,
                    &child_viewport,
                );
            });

            if self.divider {
                // Fade is driven by distance scrolled past the natural top,
                // not by total stuck distance. With a top inset, stuck
                // starts at `top` even at rest, so ramping on stuck alone
                // would snap the divider to full opacity the moment the
                // pin engages.
                let scroll_past = (viewport.y - header_bounds.y).max(0.0);
                let opacity = (scroll_past / DIVIDER_FADE_PX).clamp(0.0, 1.0);
                if opacity > 0.0 {
                    renderer.fill_quad(
                        Quad {
                            bounds: Rectangle::new(
                                Point::new(
                                    header_bounds.x,
                                    header_bounds.y + header_bounds.height + stuck,
                                ),
                                Size::new(header_bounds.width, 1.0),
                            ),
                            border: Border::default(),
                            ..Quad::default()
                        },
                        color::fade(color::border(), opacity),
                    );
                }
            }
        });
    }

    fn children(&self) -> Vec<Tree> {
        vec![Tree::new(&self.header), Tree::new(&self.body)]
    }

    fn diff(&self, tree: &mut Tree) {
        tree.diff_children(&[&self.header, &self.body]);
    }

    fn operate(
        &mut self,
        tree: &mut Tree,
        layout: Layout<'_>,
        renderer: &IcedRenderer,
        operation: &mut dyn Operation,
    ) {
        let mut children = layout.children();
        let header_layout = children.next().unwrap();
        let body_layout = children.next().unwrap();

        self.header.as_widget_mut().operate(
            &mut tree.children[0],
            header_layout,
            renderer,
            operation,
        );
        self.body.as_widget_mut().operate(
            &mut tree.children[1],
            body_layout,
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
        renderer: &IcedRenderer,
        clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
        viewport: &Rectangle,
    ) {
        let mut children = layout.children();
        let header_layout = children.next().unwrap();
        let body_layout = children.next().unwrap();

        let stuck = stuck_for(header_layout, viewport, self.top);
        let shift = Transformation::translate(0.0, -stuck);
        let header_pinned = pinned_bounds(header_layout, viewport, self.top, stuck);

        // Forward to the header first so a click on the pinned navbar
        // wins over a click on whichever body row sits behind it.
        self.header.as_widget_mut().update(
            &mut tree.children[0],
            event,
            header_layout,
            cursor * shift,
            renderer,
            clipboard,
            shell,
            &(*viewport * shift),
        );

        // Mask the body cursor when it sits on the pinned header so the
        // body row hiding behind the navbar does not also react to the
        // click.
        let body_cursor = if stuck > 0.0 && cursor.is_over(header_pinned) {
            mouse::Cursor::Unavailable
        } else {
            cursor
        };

        self.body.as_widget_mut().update(
            &mut tree.children[1],
            event,
            body_layout,
            body_cursor,
            renderer,
            clipboard,
            shell,
            viewport,
        );
    }

    fn mouse_interaction(
        &self,
        tree: &Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
        renderer: &IcedRenderer,
    ) -> mouse::Interaction {
        let mut children = layout.children();
        let header_layout = children.next().unwrap();
        let body_layout = children.next().unwrap();

        let stuck = stuck_for(header_layout, viewport, self.top);
        let shift = Transformation::translate(0.0, -stuck);
        let header_pinned = pinned_bounds(header_layout, viewport, self.top, stuck);

        if cursor.is_over(header_pinned) {
            let header = self.header.as_widget().mouse_interaction(
                &tree.children[0],
                header_layout,
                cursor * shift,
                &(*viewport * shift),
                renderer,
            );

            if !matches!(header, mouse::Interaction::None | mouse::Interaction::Idle) {
                return header;
            }

            // When pinned the header is opaque over body content sliding
            // beneath, so the body's cursor is masked. At rest the header
            // and body bounds do not overlap, so falling through to the
            // body is safe.
            if stuck > 0.0 {
                return mouse::Interaction::None;
            }
        }

        self.body.as_widget().mouse_interaction(
            &tree.children[1],
            body_layout,
            cursor,
            viewport,
            renderer,
        )
    }

    fn overlay<'b>(
        &'b mut self,
        tree: &'b mut Tree,
        layout: Layout<'b>,
        renderer: &IcedRenderer,
        viewport: &Rectangle,
        translation: Vector,
    ) -> Option<overlay::Element<'b, Message, Theme, IcedRenderer>> {
        let mut layouts = layout.children();
        let header_layout = layouts.next().unwrap();
        let body_layout = layouts.next().unwrap();

        let stuck = stuck_for(header_layout, viewport, self.top);

        // Header overlays anchor to the visually pinned position by
        // adding the stuck distance to the translation handed to them.
        // Without this, a dropdown or tooltip opened from the navbar
        // would pop up at the natural off-screen layout position.
        let header_translation = translation + Vector::new(0.0, stuck);

        let (header_slot, body_slot) = tree.children.split_at_mut(1);
        let header_tree = &mut header_slot[0];
        let body_tree = &mut body_slot[0];

        let header_overlay = self.header.as_widget_mut().overlay(
            header_tree,
            header_layout,
            renderer,
            viewport,
            header_translation,
        );
        let body_overlay = self.body.as_widget_mut().overlay(
            body_tree,
            body_layout,
            renderer,
            viewport,
            translation,
        );

        match (header_overlay, body_overlay) {
            (None, None) => None,
            (Some(one), None) | (None, Some(one)) => Some(one),
            (Some(header), Some(body)) => {
                Some(overlay::Group::with_children(vec![header, body]).overlay())
            },
        }
    }
}
