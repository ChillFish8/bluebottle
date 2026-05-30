//! A single line of typography that truncates with a trailing ellipsis when it
//! cannot fit. It wraps our custom [`Text`] widget so any typography role can
//! be turned into a truncating run. The full and truncated runs shape through
//! the same `Text::shape` path so tracking and metrics match the untruncated
//! text exactly.

use std::sync::Arc;

use iced::advanced::graphics::text::{
    self as gtext,
    Raw,
    Renderer as RawTextRenderer,
    cosmic_text,
};
use iced::advanced::widget::{Tree, tree};
use iced::advanced::{Layout, Widget, layout, renderer};
use iced::{Element, Font, Length, Rectangle, Size};

use crate::widget::text::Text;

/// The marker appended to a run that has been truncated.
const ELLIPSIS: &str = "...";

/// Wraps a typography [`Text`] so it truncates to a single line with a trailing
/// ellipsis when it overflows the available width.
pub fn ellipsis_text(text: Text<'_>) -> EllipsisText<'_> {
    EllipsisText { text }
}

/// A single line of typography that truncates with an ellipsis when it overflows. See
/// [`ellipsis_text`].
pub struct EllipsisText<'a> {
    text: Text<'a>,
}

impl<'a> EllipsisText<'a> {
    /// Sets the width the text truncates within.
    pub fn width(mut self, width: impl Into<Length>) -> Self {
        self.text = self.text.width(width);
        self
    }

    /// Sets the height of the bounding box.
    pub fn height(mut self, height: impl Into<Length>) -> Self {
        self.text = self.text.height(height);
        self
    }
}

#[derive(Default)]
struct State {
    // Owned so the weak handle passed to `fill_raw` stays alive this frame.
    buffer: Option<Arc<cosmic_text::Buffer>>,
}

impl<Message> Widget<Message, iced::Theme, iced::Renderer> for EllipsisText<'_> {
    fn size(&self) -> Size<Length> {
        Size {
            width: self.text.width,
            height: self.text.height,
        }
    }

    fn layout(
        &mut self,
        tree: &mut Tree,
        renderer: &iced::Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        let state = tree.state.downcast_mut::<State>();
        let limits = limits.width(self.text.width).height(self.text.height);
        let bounds = limits.max();
        let font = self.text.resolved_font(renderer);
        let content = self.text.content.as_ref();

        let mut font_system = gtext::font_system().write().expect("font system");
        let fs = font_system.raw();

        // Shape the whole run on a single line first. If it already fits there
        // is nothing to truncate.
        let (buffer, full) =
            self.text
                .shape(content, bounds, font, cosmic_text::Wrap::None, fs);

        let (buffer, min_bounds) = if full.width <= bounds.width {
            (buffer, full)
        } else {
            let truncated = truncate(&self.text, content, bounds, font, fs);
            self.text
                .shape(&truncated, bounds, font, cosmic_text::Wrap::None, fs)
        };

        drop(font_system);

        state.buffer = Some(Arc::new(buffer));
        layout::Node::new(limits.resolve(self.text.width, self.text.height, min_bounds))
    }

    fn draw(
        &self,
        tree: &Tree,
        renderer: &mut iced::Renderer,
        _theme: &iced::Theme,
        style: &renderer::Style,
        layout: Layout<'_>,
        _cursor: iced::mouse::Cursor,
        _viewport: &Rectangle,
    ) {
        let state = tree.state.downcast_ref::<State>();
        let Some(buffer) = &state.buffer else {
            return;
        };

        let bounds = layout.bounds();
        renderer.fill_raw(Raw {
            buffer: Arc::downgrade(buffer),
            position: bounds.position(),
            color: self.text.text_color().unwrap_or(style.text_color),
            clip_bounds: bounds,
        });
    }

    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<State>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(State::default())
    }
}

/// Finds the widest prefix of `content` that still fits once the ellipsis is
/// appended. Prefix width grows with length so the fit is monotonic and we can
/// binary search the character boundaries.
fn truncate(
    text: &Text<'_>,
    content: &str,
    bounds: Size,
    font: Font,
    font_system: &mut cosmic_text::FontSystem,
) -> String {
    let cuts: Vec<usize> = content
        .char_indices()
        .map(|(i, _)| i)
        .chain(std::iter::once(content.len()))
        .collect();

    let fitting = cuts.partition_point(|&cut| {
        let candidate = with_ellipsis(content, cut);
        let (_, size) = text.shape(
            &candidate,
            bounds,
            font,
            cosmic_text::Wrap::None,
            font_system,
        );
        size.width <= bounds.width
    });

    with_ellipsis(content, cuts[fitting.saturating_sub(1)])
}

/// Builds a truncated run. Trailing spaces and punctuation are dropped so the
/// ellipsis reads cleanly.
fn with_ellipsis(content: &str, end: usize) -> String {
    let head = content[..end].trim_end().trim_end_matches([',', '.']);
    format!("{head}{ELLIPSIS}")
}

impl<'a, Message> From<EllipsisText<'a>> for Element<'a, Message>
where
    Message: 'a,
{
    fn from(text: EllipsisText<'a>) -> Self {
        Element::new(text)
    }
}
