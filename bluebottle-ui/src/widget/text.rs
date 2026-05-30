//! Design-system typography. Each builder returns a [`Text`] widget styled for
//! one role. [`Text`] shapes through iced's own cosmic-text font system and
//! draws via `fill_raw`, which lets it carry letter spacing that iced's stock
//! text cannot. With no letter spacing set the shaping is identical to normal
//! text, kerning and ligatures intact, because cosmic-text adds the spacing to
//! each glyph advance only after shaping. Letter spacing is authored in CSS
//! pixels and divided by the font size before shaping, because cosmic-text
//! tracks in em.

use std::borrow::Cow;
use std::sync::Arc;

use iced::advanced::graphics::text::{
    self as gtext,
    Raw,
    Renderer as RawTextRenderer,
    cosmic_text,
};
use iced::advanced::text::Renderer as TextRenderer;
use iced::advanced::widget::{Tree, tree};
use iced::advanced::{Layout, Widget, layout, renderer};
use iced::widget::text::{Alignment, IntoFragment, LineHeight, Shaping};
use iced::{Color, Element, Font, Length, Pixels, Rectangle, Size};

use crate::{color, font};

/// Display Title Large
///
/// Hero & player titles. One per screen. The single largest thing in view. Tight tracking, line-height near 1.
pub fn display_large<'a>(input: impl IntoFragment<'a>) -> Text<'a> {
    Text::new(input)
        .font(font::bold())
        .size(52)
        .line_height(1.5)
        .letter_spacing(-1.0)
        .color(color::TEXT_PRIMARY)
}

/// Display Title Medium
///
/// Hero & player titles. One per screen. The single largest thing in view. Tight tracking, line-height near 1.
pub fn display_medium<'a>(input: impl IntoFragment<'a>) -> Text<'a> {
    Text::new(input)
        .font(font::bold())
        .size(44)
        .line_height(1.5)
        .letter_spacing(-0.5)
        .color(color::TEXT_PRIMARY)
}

/// Heading Large
///
/// Drawer tiles, episode names, top search result. The largest text inside a panel or overlay.
pub fn heading_large<'a>(input: impl IntoFragment<'a>) -> Text<'a> {
    Text::new(input)
        .font(font::bold())
        .size(26)
        .line_height(1.15)
        .letter_spacing(-0.3)
        .color(color::TEXT_PRIMARY)
}

/// Header Medium
///
/// Drawer tiles, episode names, top search result. The largest text inside a panel or overlay.
pub fn heading_medium<'a>(input: impl IntoFragment<'a>) -> Text<'a> {
    Text::new(input)
        .font(font::bold())
        .size(22)
        .line_height(1.2)
        .letter_spacing(-0.2)
        .color(color::TEXT_PRIMARY)
}

/// Title Small
///
/// Drawer tiles, episode names, top search result. The largest text inside a panel or overlay.
pub fn title_small<'a>(input: impl IntoFragment<'a>) -> Text<'a> {
    Text::new(input)
        .font(font::bold())
        .size(18)
        .letter_spacing(-0.2)
        .color(color::TEXT_PRIMARY)
}

#[derive(Debug, Eq, PartialEq, Copy, Clone, Default)]
/// The context decides what variant the text should form to if
/// the given text has alt variants.
pub enum Variant {
    #[default]
    /// Primary format.
    Main,
    /// Secondary format.
    Alt,
}

/// Subtitle
///
/// Supporting lines beneath a title, search input, and reading copy. Lighter weights, relaxed line-height.
pub fn subtitle<'a>(input: impl IntoFragment<'a>, ctx: Variant) -> Text<'a> {
    let color = match ctx {
        Variant::Main => color::TEXT_PRIMARY,
        Variant::Alt => color::TEXT_SECONDARY,
    };

    Text::new(input)
        .font(font::medium())
        .size(18)
        .line_height(1.4)
        .color(color)
}

/// Lead
///
/// Supporting lines beneath a title, search input, and reading copy. Lighter weights, relaxed line-height.
pub fn lead<'a>(input: impl IntoFragment<'a>, ctx: Variant) -> Text<'a> {
    let color = match ctx {
        Variant::Main => color::with_alpha(color::TEXT_PRIMARY, 0.78),
        Variant::Alt => color::TEXT_SECONDARY,
    };

    Text::new(input)
        .font(font::regular())
        .size(16)
        .line_height(1.6)
        .color(color)
}

/// Body
pub fn body<'a>(input: impl IntoFragment<'a>) -> Text<'a> {
    Text::new(input)
        .font(font::regular())
        .size(14)
        .line_height(1.6)
        .letter_spacing(0.0)
        .color(color::with_alpha(color::TEXT_PRIMARY, 0.82))
}

/// Section Heading
///
/// Supporting lines beneath a title, search input, and reading copy. Lighter weights, relaxed line-height.
pub fn section_heading<'a>(input: impl IntoFragment<'a>) -> Text<'a> {
    Text::new(input)
        .font(font::bold())
        .size(14)
        .letter_spacing(0.2)
        .color(color::TEXT_PRIMARY)
}

/// Card Title
///
/// The interface's center of gravity. Card titles, list rows, buttons, queue items. Most text lives here.
pub fn card_title<'a>(input: impl IntoFragment<'a>) -> Text<'a> {
    Text::new(input).size(13).color(color::TEXT_SECONDARY)
}

/// Hero Label
///
/// Mirrors [card_title] but a stronger weight for hero buttons.
pub fn hero_label<'a>(input: impl IntoFragment<'a>) -> Text<'a> {
    Text::new(input).size(13).font(font::bold())
}

/// Label
///
/// Design note: When inactive, labels should use [Variant::Alt].
///
/// The interface's center of gravity. Card titles, list rows, buttons, queue items. Most text lives here.
pub fn label<'a>(input: impl IntoFragment<'a>, ctx: Variant) -> Text<'a> {
    let color = match ctx {
        Variant::Main => color::TEXT_PRIMARY,
        Variant::Alt => color::TEXT_SECONDARY,
    };

    Text::new(input).size(12).color(color)
}

/// Caption
///
/// The smallest text. Sub-captions, counts, badges and the all-caps eyebrows
/// that label every section.
pub fn caption<'a>(input: impl IntoFragment<'a>) -> Text<'a> {
    Text::new(input)
        .font(font::regular())
        .size(11)
        .color(color::TEXT_SECONDARY)
}

/// Eyebrow
///
/// Design note: should be all caps ALWAYS.
///
/// The smallest text. Sub-captions, counts, badges and the all-caps eyebrows
/// that label every section.
pub fn eyebrow<'a>(input: impl IntoFragment<'a>, ctx: Variant) -> Text<'a> {
    let color = match ctx {
        Variant::Main => color::primary(),
        Variant::Alt => color::TEXT_SECONDARY,
    };

    let size = match ctx {
        Variant::Main => 10,
        Variant::Alt => 11,
    };

    let font = match ctx {
        Variant::Main => font::bold(),
        Variant::Alt => font::semibold(),
    };

    Text::new(input)
        .font(font)
        .size(size)
        .letter_spacing(0.5)
        .color(color)
}

/// Micro Label
///
/// For field labels, meta tags.
pub fn micro_label<'a>(input: impl IntoFragment<'a>) -> Text<'a> {
    Text::new(input)
        .font(font::regular())
        .size(10)
        .letter_spacing(0.4)
}

/// Lyric
///
/// Note: Main refers to the active lyric (full opacity) and alt is for queued entries.
pub fn lyric<'a>(input: impl IntoFragment<'a>, ctx: Variant) -> Text<'a> {
    let color = match ctx {
        Variant::Main => color::TEXT_PRIMARY,
        Variant::Alt => color::with_alpha(color::TEXT_PRIMARY, 0.5),
    };

    Text::new(input)
        .font(font::semibold())
        .size(22)
        .letter_spacing(-0.2)
        .color(color)
}

/// Media Overlay
pub fn media_overlay<'a>(input: impl IntoFragment<'a>) -> Text<'a> {
    Text::new(input)
        .font(font::bold())
        .size(10)
        .letter_spacing(1.5)
        .color(color::with_alpha(color::TEXT_PRIMARY, 0.78))
}

/// A styled run of text. Built by the typography functions in this module.
pub struct Text<'a> {
    content: Cow<'a, str>,
    size: Pixels,
    line_height: LineHeight,
    font: Option<Font>,
    color: Option<Color>,
    letter_spacing: f32,
    align_x: Alignment,
    width: Length,
    height: Length,
}

impl<'a> Text<'a> {
    fn new(content: impl IntoFragment<'a>) -> Self {
        Self {
            content: content.into_fragment(),
            size: Pixels(14.0),
            line_height: LineHeight::default(),
            font: None,
            color: None,
            letter_spacing: 0.0,
            align_x: Alignment::Default,
            width: Length::Shrink,
            height: Length::Shrink,
        }
    }

    /// Sets the text size.
    pub fn size(mut self, size: impl Into<Pixels>) -> Self {
        self.size = size.into();
        self
    }

    /// Sets the line height, relative to the size unless made absolute.
    pub fn line_height(mut self, line_height: impl Into<LineHeight>) -> Self {
        self.line_height = line_height.into();
        self
    }

    /// Sets the font. Defaults to the renderer's default font.
    pub fn font(mut self, font: impl Into<Font>) -> Self {
        self.font = Some(font.into());
        self
    }

    /// Sets the colour. Left unset the text inherits the cascaded text colour,
    /// so it can ride a parent's animated `text_color`.
    pub fn color(mut self, color: Color) -> Self {
        self.color = Some(color);
        self
    }

    /// Sets the extra tracking between glyphs in pixels. This matches CSS
    /// letter-spacing. The value is converted to em before shaping because
    /// cosmic-text tracks in em. Zero leaves shaping untouched.
    pub fn letter_spacing(mut self, letter_spacing: f32) -> Self {
        self.letter_spacing = letter_spacing;
        self
    }

    /// Sets the horizontal alignment within the text bounds.
    pub fn align_x(mut self, align_x: impl Into<Alignment>) -> Self {
        self.align_x = align_x.into();
        self
    }

    /// Sets the width of the text's bounding box.
    pub fn width(mut self, width: impl Into<Length>) -> Self {
        self.width = width.into();
        self
    }

    /// Sets the height of the text's bounding box.
    pub fn height(mut self, height: impl Into<Length>) -> Self {
        self.height = height.into();
        self
    }

    /// The explicit colour, if one was set. `None` means the text inherits the
    /// cascade. Lets a wrapping widget match an affordance to the text.
    pub fn text_color(&self) -> Option<Color> {
        self.color
    }
}

#[derive(Default)]
struct State {
    // Owned so the weak handle passed to `fill_raw` stays alive this frame.
    buffer: Option<Arc<cosmic_text::Buffer>>,
}

impl<'a, Message> Widget<Message, iced::Theme, iced::Renderer> for Text<'a> {
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
        let limits = limits.width(self.width).height(self.height);
        let bounds = limits.max();
        let font = self.font.unwrap_or_else(|| renderer.default_font());

        let mut font_system = gtext::font_system().write().expect("font system");
        let mut buffer = cosmic_text::Buffer::new(
            font_system.raw(),
            cosmic_text::Metrics::new(
                self.size.0,
                self.line_height.to_absolute(self.size).0,
            ),
        );

        buffer.set_size(font_system.raw(), Some(bounds.width), Some(bounds.height));

        // Letter spacing rides on the attrs so the shaper adds it per glyph
        // after kerning. Left at zero the shaping is identical to plain text.
        // cosmic-text tracks in em so divide our CSS px value by the font
        // size to match what a browser draws.
        let attrs = gtext::to_attributes(font);
        let attrs = if self.letter_spacing == 0.0 {
            attrs
        } else {
            attrs.letter_spacing(self.letter_spacing / self.size.0)
        };

        buffer.set_text(
            font_system.raw(),
            &self.content,
            &attrs,
            gtext::to_shaping(Shaping::Advanced, &self.content),
            None,
        );

        let min_bounds = gtext::align(&mut buffer, font_system.raw(), self.align_x);
        drop(font_system);

        state.buffer = Some(Arc::new(buffer));
        layout::Node::new(limits.resolve(self.width, self.height, min_bounds))
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
            color: self.color.unwrap_or(style.text_color),
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

impl<'a, Message> From<Text<'a>> for Element<'a, Message>
where
    Message: 'a,
{
    fn from(text: Text<'a>) -> Self {
        Element::new(text)
    }
}
