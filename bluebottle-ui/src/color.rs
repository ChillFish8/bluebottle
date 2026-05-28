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
pub const DISABLED: iced::Color = iced::color!(0x364153);

// Text secondary colors
pub const TEXT_DEFAULT: iced::Color = iced::color!(0xFFFFFF);
pub const TEXT_PRIMARY: iced::Color = PRIMARY;
pub const TEXT_SECONDARY: iced::Color = iced::color!(0x62748E);
/// A muted text colour. Sits between [`TEXT_DEFAULT`] and [`TEXT_SECONDARY`]
/// so de-emphasised text still reads as legible foreground.
pub const TEXT_MUTED: iced::Color = mix(TEXT_SECONDARY, TEXT_DEFAULT, 0.35);
pub const TEXT_DARK: iced::Color = DISABLED;
pub const TEXT_DARKER: iced::Color = HOVER_HIGHLIGHT;

// Special edge cases to be used _very_ sparingly
pub const SHIMMER: iced::Color = iced::color!(0x182236);

// Derived shades. The single source of truth for tinted surfaces and accents,
// so the same colour is never recomputed ad hoc across the app.

/// An elevated overlay surface. The background leaned slightly toward primary so
/// panels (e.g. the sidebar) read as a distinct shade from the page.
pub const SURFACE: iced::Color = mix(BACKGROUND, PRIMARY, 0.10);

/// Leading-edge accent line on overlay panels. The primary darkened and held at
/// low opacity.
pub const BORDER: iced::Color = with_alpha(scale(PRIMARY, 0.6), 0.4);

/// Full-screen wash dimming content behind an overlay. Its alpha is the wash at
/// full reveal. Callers scale it by the overlay's reveal animation.
pub const VEIL: iced::Color = with_alpha(BACKGROUND, 0.55);

/// Glow color for the image-less background gradient. The primary tinted down
/// so it reads as a soft wash rather than a high-contrast highlight.
pub const GLOW: iced::Color = scale(PRIMARY, 0.4);

/// The overlay scrollbar when shown, a mid-gray.
pub const SCROLLBAR: iced::Color = iced::color!(0x64748B);

/// The scrollbar while hovered or grabbed, a lighter gray for contrast.
pub const SCROLLBAR_HOVER: iced::Color = iced::color!(0x94A3B8);

/// Linearly interpolates `from` → `to` by `t` in `[0, 1]`, in sRGB components.
pub const fn mix(from: iced::Color, to: iced::Color, t: f32) -> iced::Color {
    iced::Color {
        r: from.r + (to.r - from.r) * t,
        g: from.g + (to.g - from.g) * t,
        b: from.b + (to.b - from.b) * t,
        a: from.a + (to.a - from.a) * t,
    }
}

/// Scales a colour's rgb toward black by `factor` (`1.0` = unchanged), alpha kept.
pub const fn scale(color: iced::Color, factor: f32) -> iced::Color {
    iced::Color {
        r: color.r * factor,
        g: color.g * factor,
        b: color.b * factor,
        a: color.a,
    }
}

/// Copies `color` with a replaced `alpha`.
pub const fn with_alpha(color: iced::Color, alpha: f32) -> iced::Color {
    iced::Color {
        r: color.r,
        g: color.g,
        b: color.b,
        a: alpha,
    }
}

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
