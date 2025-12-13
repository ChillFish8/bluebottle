use std::borrow::Cow;

use iced::Font;
use iced::font::{Family, Weight};

/// Noto Sans English & Latin
static NOTO_DEFAULT: &'static [u8] =
    include_bytes!("../assets/Noto/NotoSans-VariableFont_wdth,wght.ttf");
/// Noto Sans Japanese
static NOTO_JP: &'static [u8] = include_bytes!("../assets/Noto/NotoSansJP-VariableFont_wght.ttf");
/// Noto Sans Korean
static NOTO_KR: &'static [u8] = include_bytes!("../assets/Noto/NotoSansKR-VariableFont_wght.ttf");
/// Noto Sans Traditional Chinese
static NOTO_TC: &'static [u8] = include_bytes!("../assets/Noto/NotoSansTC-VariableFont_wght.ttf");
/// Noto Sans Simplified Chinese
static NOTO_SC: &'static [u8] = include_bytes!("../assets/Noto/NotoSansSC-VariableFont_wght.ttf");

/// Returns a vector containing the embedded noto font data.
pub fn noto_fonts() -> Vec<Cow<'static, [u8]>> {
    vec![
        Cow::Borrowed(NOTO_DEFAULT),
        Cow::Borrowed(NOTO_JP),
        Cow::Borrowed(NOTO_KR),
        Cow::Borrowed(NOTO_TC),
        Cow::Borrowed(NOTO_SC),
    ]
}

/// Use the default (Noto) font with regular weighting.
pub const fn regular() -> Font {
    let mut font = Font::DEFAULT;
    font.family = Family::SansSerif;
    font.weight = Weight::Normal;
    font
}

/// Use the default (Noto) font with semibold weighting.
pub const fn semibold() -> Font {
    let mut font = Font::DEFAULT;
    font.family = Family::SansSerif;
    font.weight = Weight::Semibold;
    font
}

/// Use the default (Noto) font with bold weighting.
pub const fn bold() -> Font {
    let mut font = Font::DEFAULT;
    font.family = Family::SansSerif;
    font.weight = Weight::Bold;
    font
}
