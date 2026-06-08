//! Renderer helpers shared by the card chassis and its row siblings.

use iced::advanced::text::{Renderer as TextRenderer, Text as AdvText};
use iced::widget::text::{Alignment as TextAlign, LineHeight, Shaping, Wrapping};
use iced::{Color, Pixels, Point, Rectangle, Renderer, alignment};

use crate::icon;

/// Renders a centred Material Icon glyph inside `area`. A no-op if the area
/// or glyph would clip below a pixel so the shaper never sees zero bounds.
pub(super) fn paint_centered_icon(
    renderer: &mut Renderer,
    name: &str,
    area: Rectangle,
    glyph_size: f32,
    fill: Color,
) {
    if glyph_size < 1.0 || area.width < 1.0 || area.height < 1.0 {
        return;
    }

    let text = AdvText {
        content: icon::filled_codepoint(name).to_string(),
        bounds: area.size(),
        size: Pixels(glyph_size),
        line_height: LineHeight::Relative(1.0),
        font: icon::ICON_FILLED_FONT,
        align_x: TextAlign::Center,
        align_y: alignment::Vertical::Center,
        shaping: Shaping::Advanced,
        wrapping: Wrapping::None,
    };

    let anchor = Point::new(area.x + area.width * 0.5, area.y + area.height * 0.5);
    TextRenderer::fill_text(renderer, text, anchor, fill, area);
}
