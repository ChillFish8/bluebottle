//! Material Icons

use std::borrow::Cow;

use iced::advanced::text;
use iced::font::{Family, Font, Stretch, Style, Weight};
use iced::widget::Text;
use iced::widget::text::Catalog;

include!(concat!(env!("OUT_DIR"), "/icons_lut.rs"));

const ICON_FILLED_FONT: Font = Font {
    family: Family::Name("Material Icons"),
    weight: Weight::Normal,
    stretch: Stretch::Normal,
    style: Style::Normal,
};
const ICON_OUTLINED_FONT: Font = Font {
    family: Family::Name("Material Icons Outlined"),
    weight: Weight::Normal,
    stretch: Stretch::Normal,
    style: Style::Normal,
};

static ICONS_OUTLINE_FONT_DATA: &[u8] =  include_bytes!("../assets/MaterialIcons/MaterialIconsOutlined-Regular.otf");
static ICONS_FILLED_FONT_DATA: &[u8] =  include_bytes!("../assets/MaterialIcons/MaterialIcons-Regular.ttf");

pub(crate) fn extend_fonts(fonts: &mut Vec<Cow<'static, [u8]>>) {
    fonts.push(Cow::Borrowed(ICONS_OUTLINE_FONT_DATA));
    fonts.push(Cow::Borrowed(ICONS_FILLED_FONT_DATA));
}

/// Returns an iterator over all icons.
pub fn all() -> impl Iterator<Item = &'static str> {
    OUTLINE_ICON_CODEPOINTS.keys().map(|k| &**k)
        .chain(FILLED_ICON_CODEPOINTS.keys().map(|k| &**k))
}

/// Checks if a given icon name exists.
pub fn exists(name: &str) -> bool {
    OUTLINE_ICON_CODEPOINTS.contains_key(name)
        || FILLED_ICON_CODEPOINTS.contains_key(name)
}

/// Get a new icon widget in the outline format.
pub fn outline<Theme, Renderer>(name: &str) -> Text<'static, Theme, Renderer>
where
    Theme: Catalog,
    Renderer: text::Renderer,
    Renderer::Font: From<Font>,
{
    let code = OUTLINE_ICON_CODEPOINTS
        .get(name)
        .map(|c| &**c)
        .unwrap_or_else(|| panic!("icon {name:?} does not exist"));
    Text::new(code)
        .size(24)
        .font(ICON_OUTLINED_FONT)
}

/// Get a new icon widget in the filled format.
pub fn filled<Theme, Renderer>(name: &str) -> Text<'static, Theme, Renderer>
where
    Theme: Catalog,
    Renderer: text::Renderer,
    Renderer::Font: From<Font>,
{
    let code = FILLED_ICON_CODEPOINTS
        .get(name)
        .map(|c| &**c)
        .unwrap_or_else(|| panic!("icon {name:?} does not exist"));

    Text::new(code)
        .size(48)
        .font(ICON_FILLED_FONT)
}
