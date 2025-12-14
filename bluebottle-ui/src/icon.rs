//! Material Icons

use std::borrow::Cow;

use iced::advanced::text;
use iced::font::{Family, Font, Stretch, Style, Weight};
use iced::widget::Text;
use iced::widget::text::Catalog;

include!(concat!(env!("OUT_DIR"), "/icons_lut.rs"));

static ICONS_FONT_DATA: &[u8] =
    include_bytes!("../assets/MaterialIcons/MaterialSymbolsRounded[FILL,GRAD,opsz,wght].ttf");
const ICON_FILLED_FONT: Font = Font {
    family: Family::Name("Material Symbols Rounded"),
    weight: Weight::Bold,
    stretch: Stretch::Normal,
    style: Style::Normal,
};
const ICON_OUTLINED_FONT: Font = Font {
    family: Family::Name("Material Symbols Rounded"),
    weight: Weight::Normal,
    stretch: Stretch::Normal,
    style: Style::Normal,
};

pub(crate) fn extend_fonts(fonts: &mut Vec<Cow<'static, [u8]>>) {
    fonts.push(Cow::Borrowed(ICONS_FONT_DATA));
}

/// Returns an iterator over all icons.
pub fn all() -> impl Iterator<Item = &'static str> {
    ICON_CODEPOINTS.keys().map(|k| &**k)
}

/// Checks if a given icon name exists.
pub fn exists(name: &str) -> bool {
    ICON_CODEPOINTS.contains_key(name)
}

/// Get a new icon widget in the outline format.
pub fn outline<Theme, Renderer>(name: &str) -> Text<'static, Theme, Renderer>
where
    Theme: Catalog,
    Renderer: text::Renderer,
    Renderer::Font: From<Font>,
{
    let code = ICON_CODEPOINTS
        .get(name)
        .map(|c| &**c)
        .unwrap_or_else(|| panic!("icon {name:?} does not exist"));
    Text::new(code).font(ICON_OUTLINED_FONT)
}

/// Get a new icon widget in the filled format.
pub fn filled<Theme, Renderer>(name: &str) -> Text<'static, Theme, Renderer>
where
    Theme: Catalog,
    Renderer: text::Renderer,
    Renderer::Font: From<Font>,
{
    let code = ICON_CODEPOINTS
        .get(name)
        .map(|c| &**c)
        .unwrap_or_else(|| panic!("icon {name:?} does not exist"));

    Text::new(code).font(ICON_FILLED_FONT)
}
