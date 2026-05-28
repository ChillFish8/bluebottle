use std::borrow::Cow;

use iced::Font;
use iced::font::{Family, Weight};

/// Body text size for primary content. Bar labels, breadcrumbs, anything that
/// needs to read like running text.
pub const TEXT_LARGE: f32 = 16.0;

/// Body text size for compact rows. Card labels, pills, ratings, navigation.
pub const TEXT_MEDIUM: f32 = 14.0;

/// Body text size for captions and de-emphasised meta lines. Form labels,
/// pill captions, card sub-labels.
pub const TEXT_SMALL: f32 = 12.0;

/// Top-level heading size. Screen titles and bar titles.
pub const HEADING_1: f32 = 20.0;

/// Section heading size. Subheadings inside a screen.
pub const HEADING_2: f32 = 18.0;

/// Minor heading size. Card group titles and inline headings.
pub const HEADING_3: f32 = 16.0;

/// Noto Sans English & Latin
static NOTO_DEFAULT: &'static [u8] =
    include_bytes!("../assets/Noto/NotoSans-VariableFont_wdth,wght.ttf");
/// Noto Sans Japanese
static NOTO_JP: &'static [u8] =
    include_bytes!("../assets/Noto/NotoSansJP-VariableFont_wght.ttf");
/// Noto Sans Korean
static NOTO_KR: &'static [u8] =
    include_bytes!("../assets/Noto/NotoSansKR-VariableFont_wght.ttf");
/// Noto Sans Traditional Chinese
static NOTO_TC: &'static [u8] =
    include_bytes!("../assets/Noto/NotoSansTC-VariableFont_wght.ttf");
/// Noto Sans Simplified Chinese
static NOTO_SC: &'static [u8] =
    include_bytes!("../assets/Noto/NotoSansSC-VariableFont_wght.ttf");

/// Returns a vector containing the embedded font data for the UI.
pub fn required_fonts() -> Vec<Cow<'static, [u8]>> {
    let mut base = vec![
        Cow::Borrowed(NOTO_DEFAULT),
        Cow::Borrowed(NOTO_JP),
        Cow::Borrowed(NOTO_KR),
        Cow::Borrowed(NOTO_TC),
        Cow::Borrowed(NOTO_SC),
    ];
    // Add icon font file.
    crate::icon::extend_fonts(&mut base);
    base
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
