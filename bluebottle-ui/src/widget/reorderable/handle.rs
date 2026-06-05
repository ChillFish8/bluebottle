//! Grab handle marker. A transparent pass-through widget whose only role is
//! to advertise its layout bounds to an enclosing [`reorderable`] parent.
//! The wrapped content renders exactly as if it were not wrapped.

use iced::advanced::widget::{Operation, Tree, tree};
use iced::advanced::{Clipboard, Layout, Renderer, Shell, Widget, layout, overlay};
use iced::{Element, Event, Length, Rectangle, Size, Vector, mouse};

/// Wraps `content` as the grab affordance for an enclosing [`reorderable`]
/// row. The wrapped content renders exactly as if it were not wrapped.
pub fn grab_handle<'a, Message, Theme, R>(
    content: impl Into<Element<'a, Message, Theme, R>>,
) -> Element<'a, Message, Theme, R>
where
    Message: 'a,
    Theme: 'a,
    R: 'a + Renderer,
{
    Element::new(GrabHandle {
        content: content.into(),
    })
}

/// The state type whose [`tree::Tag`] uniquely identifies grab handles in a
/// child tree walk.
pub(super) struct Marker;

/// The shared tag that [`reorderable`]'s tree walk matches against to locate
/// each row's handle bounds.
pub(super) fn tag() -> tree::Tag {
    tree::Tag::of::<Marker>()
}

/// Walks `tree` and its layout in lockstep, returning the absolute bounds of
/// the first grab handle marker encountered. Returns `None` when no marker
/// sits inside the subtree, in which case the row is never the source of a
/// drag.
///
/// Some widgets (notably [`iced::widget::Container`]) are tree-transparent
/// (their tag and children delegate to the wrapped content) but still wrap
/// the content's layout in an extra node for padding and alignment. When the
/// walk spots such a single-child layout mismatch it descends one layout
/// level while keeping the tree at the same position so the children line up
/// again on the next step.
pub(super) fn find_in(tree: &Tree, layout: Layout<'_>) -> Option<Rectangle> {
    if tree.tag == tag() {
        return Some(layout.bounds());
    }

    let layout_children: Vec<Layout> = layout.children().collect();

    // Tree-transparent layout wrapper: an ancestor (e.g. a Container) added
    // one layout level on top of its content but delegated its tree to the
    // content. Descend one layout step while keeping the tree at this level
    // so the next recursion zips them aligned. Only triggered when the tree
    // genuinely has multiple children to align, so a leaf with zero tree
    // children can never recurse without making progress.
    if tree.children.len() > 1 && layout_children.len() == 1 {
        return find_in(tree, layout_children[0]);
    }

    for (sub_tree, sub_layout) in tree.children.iter().zip(layout_children) {
        if let Some(rect) = find_in(sub_tree, sub_layout) {
            return Some(rect);
        }
    }
    None
}

struct GrabHandle<'a, Message, Theme, R> {
    content: Element<'a, Message, Theme, R>,
}

impl<Message, Theme, R> Widget<Message, Theme, R> for GrabHandle<'_, Message, Theme, R>
where
    R: Renderer,
{
    fn size(&self) -> Size<Length> {
        self.content.as_widget().size()
    }

    fn size_hint(&self) -> Size<Length> {
        self.content.as_widget().size_hint()
    }

    fn layout(
        &mut self,
        tree: &mut Tree,
        renderer: &R,
        limits: &layout::Limits,
    ) -> layout::Node {
        self.content
            .as_widget_mut()
            .layout(&mut tree.children[0], renderer, limits)
    }

    fn draw(
        &self,
        tree: &Tree,
        renderer: &mut R,
        theme: &Theme,
        style: &iced::advanced::renderer::Style,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
    ) {
        self.content.as_widget().draw(
            &tree.children[0],
            renderer,
            theme,
            style,
            layout,
            cursor,
            viewport,
        );
    }

    fn tag(&self) -> tree::Tag {
        tag()
    }

    fn state(&self) -> tree::State {
        tree::State::new(Marker)
    }

    fn children(&self) -> Vec<Tree> {
        vec![Tree::new(&self.content)]
    }

    fn diff(&self, tree: &mut Tree) {
        tree.diff_children(std::slice::from_ref(&self.content));
    }

    fn operate(
        &mut self,
        tree: &mut Tree,
        layout: Layout<'_>,
        renderer: &R,
        operation: &mut dyn Operation,
    ) {
        self.content.as_widget_mut().operate(
            &mut tree.children[0],
            layout,
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
        renderer: &R,
        clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
        viewport: &Rectangle,
    ) {
        self.content.as_widget_mut().update(
            &mut tree.children[0],
            event,
            layout,
            cursor,
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
        renderer: &R,
    ) -> mouse::Interaction {
        self.content.as_widget().mouse_interaction(
            &tree.children[0],
            layout,
            cursor,
            viewport,
            renderer,
        )
    }

    fn overlay<'b>(
        &'b mut self,
        tree: &'b mut Tree,
        layout: Layout<'b>,
        renderer: &R,
        viewport: &Rectangle,
        translation: Vector,
    ) -> Option<overlay::Element<'b, Message, Theme, R>> {
        self.content.as_widget_mut().overlay(
            &mut tree.children[0],
            layout,
            renderer,
            viewport,
            translation,
        )
    }
}
