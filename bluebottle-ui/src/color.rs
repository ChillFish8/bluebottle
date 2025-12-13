use std::sync::Arc;

use iced::theme::{Custom, Palette};

// "Core" colours
pub const PRIMARY: iced::Color = iced::color!(0x615FFF);
pub const SUCCESS: iced::Color = iced::color!(0x00BC7D);
pub const ERROR: iced::Color = iced::color!(0xFF2056);
pub const WARNING: iced::Color = iced::color!(0xFE9A00);

// Background colors
pub const BACKGROUND: iced::Color = iced::color!(0x101828);
pub const SECONDARY: iced::Color = iced::color!(0x162034);
pub const HOVER_HIGHLIGHT: iced::Color = iced::color!(0x1E2939);

// Text secondary colors
pub const TEXT_DEFAULT: iced::Color = iced::color!(0xFFFFFF);
pub const TEXT_SECONDARY: iced::Color = iced::color!(0x62748E);
pub const TEXT_DARK: iced::Color = iced::color!(0x364153);
pub const TEXT_DARKER: iced::Color = HOVER_HIGHLIGHT;

/// Returns a configured color theme for an iced application.
pub fn theme() -> iced::theme::Theme {
    let base_palette = Palette {
        background: BACKGROUND,
        text: TEXT_DEFAULT,
        primary: PRIMARY,
        success: SUCCESS,
        warning: WARNING,
        danger: ERROR,
    };

    let custom = Custom::new("Bluebottle".into(), base_palette);
    iced::theme::Theme::Custom(Arc::new(custom))
}
