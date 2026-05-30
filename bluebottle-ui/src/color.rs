use std::sync::Arc;
use std::sync::atomic::{AtomicU8, Ordering};

use iced::Color;
use iced::theme::{Custom, Palette, Theme};

// Neutrals. Fixed across every accent.

/// Deepest level. App and page background behind all surfaces.
pub const CANVAS: Color = iced::color!(0x0A0E18);

/// Primary panel background. Base layer of most screens.
pub const BG: Color = iced::color!(0x101828);

/// Raised cards, meta panels, and grouped containers.
pub const SECONDARY: Color = iced::color!(0x162034);

/// Hover and selected fills. Also the darkest text/icon shade.
pub const HOVER: Color = iced::color!(0x1E2939);

/// Pure white.
pub const WHITE: Color = iced::color!(0xFFFFFF);

/// Darkest text shade. Alias of [`HOVER`].
pub const TEXT_DARKER: Color = HOVER;

/// Dividing dots, tertiary metadata, low-emphasis text.
pub const TEXT_DARK: Color = iced::color!(0x45556C);

/// Secondary copy, captions, inactive nav labels.
pub const TEXT_SECONDARY: Color = iced::color!(0x62748E);

/// Headings and primary reading text.
pub const TEXT_PRIMARY: Color = iced::color!(0xFFFFFF);

/// A muted text colour. Sits between [`TEXT_PRIMARY`] and [`TEXT_SECONDARY`]
/// so de-emphasised text still reads as legible foreground.
pub const TEXT_MUTED: Color = mix(TEXT_SECONDARY, TEXT_PRIMARY, 0.35);

// Glass and scrim. Fixed across every accent.

/// Top of the inspect-drawer gradient.
pub const GLASS_TOP: Color =
    Color::from_rgba(28.0 / 255.0, 22.0 / 255.0, 60.0 / 255.0, 0.92);

/// Bottom of the inspect-drawer gradient. Sticky tab bar fill.
pub const GLASS_BASE: Color =
    Color::from_rgba(20.0 / 255.0, 18.0 / 255.0, 42.0 / 255.0, 0.96);

/// Dimming wash behind drawers and modals.
pub const SCRIM: Color = Color::from_rgba(8.0 / 255.0, 10.0 / 255.0, 20.0 / 255.0, 0.55);

// Alpha hairlines. Fixed across every accent. The opacities are authored in
// sRGB and run through `srgb_alpha` so they render at the weight the design
// reads. See [`srgb_alpha`].

/// Default hairline between surfaces. sRGB 6%.
pub fn border() -> Color {
    with_alpha(WHITE, srgb_alpha(0.06))
}

/// Emphasised outlines and glass-button borders. sRGB 10%.
pub fn border_strong() -> Color {
    with_alpha(WHITE, srgb_alpha(0.10))
}

/// Subtle row hover over dark surfaces. sRGB 4%.
pub fn hover_veil() -> Color {
    with_alpha(WHITE, srgb_alpha(0.04))
}

// Fixed semantics outside the accent quartet.

/// Star ratings and review scores.
pub const GOLD: Color = iced::color!(0xFACC15);

// Scrollbar tints. Carved out of the spec on purpose so the bar keeps its
// hover-brighten.

/// The overlay scrollbar when shown, a mid-gray.
pub const SCROLLBAR: Color = iced::color!(0x64748B);

/// The scrollbar while hovered or grabbed, a lighter gray for contrast.
pub const SCROLLBAR_HOVER: Color = iced::color!(0x94A3B8);

// Accent runtime. The brand quartet and anything derived from it lives behind
// `current_accent()` so the picker can swap the whole palette mid-session.

/// Selectable accent theme. Defaults to [`Accent::Default`] on launch.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum Accent {
    Default = 0,
    Pastel = 1,
    Electric = 2,
    Candy = 3,
}

/// The four colours that vary per accent.
struct AccentSet {
    primary: Color,
    success: Color,
    error: Color,
    warning: Color,
}

const ACCENTS: [AccentSet; 4] = [
    AccentSet {
        primary: iced::color!(0x615FFF),
        success: iced::color!(0x00BC7D),
        error: iced::color!(0xFF2056),
        warning: iced::color!(0xFE9A00),
    },
    AccentSet {
        primary: iced::color!(0x7DD3FC),
        success: iced::color!(0x34D399),
        error: iced::color!(0xF472B6),
        warning: iced::color!(0xFBBF24),
    },
    AccentSet {
        primary: iced::color!(0xA78BFA),
        success: iced::color!(0x22D3EE),
        error: iced::color!(0xFB7185),
        warning: iced::color!(0xFACC15),
    },
    AccentSet {
        primary: iced::color!(0xF472B6),
        success: iced::color!(0x10B981),
        error: iced::color!(0x60A5FA),
        warning: iced::color!(0xF59E0B),
    },
];

static ACCENT: AtomicU8 = AtomicU8::new(Accent::Default as u8);

/// The active accent. Reads the global atomic and decodes.
pub fn current_accent() -> Accent {
    match ACCENT.load(Ordering::Relaxed) {
        1 => Accent::Pastel,
        2 => Accent::Electric,
        3 => Accent::Candy,
        _ => Accent::Default,
    }
}

/// Sets the active accent. Repaint happens on the next iced frame, which fires
/// as soon as any message lands.
pub fn set_accent(accent: Accent) {
    ACCENT.store(accent as u8, Ordering::Relaxed);
}

fn active() -> &'static AccentSet {
    &ACCENTS[current_accent() as usize]
}

/// Brand colour for the active accent. Buttons, active tabs, focus.
pub fn primary() -> Color {
    active().primary
}

/// Tinted backdrop behind primary or selected items. sRGB 18%.
pub fn primary_soft() -> Color {
    with_alpha(primary(), srgb_alpha(0.18))
}

/// Watched, completed, positive confirmation.
pub fn success() -> Color {
    active().success
}

/// Destructive actions. The rotten-tomato score.
pub fn error() -> Color {
    active().error
}

/// Cautions and pending states.
pub fn warning() -> Color {
    active().warning
}

/// Glow colour for the image-less background gradient. The primary tinted down
/// so it reads as a soft wash rather than a high-contrast highlight.
pub fn glow() -> Color {
    scale(primary(), 0.4)
}

/// Linearly interpolates `from` to `to` by `t` in `[0, 1]`, in sRGB components.
pub const fn mix(from: Color, to: Color, t: f32) -> Color {
    Color {
        r: from.r + (to.r - from.r) * t,
        g: from.g + (to.g - from.g) * t,
        b: from.b + (to.b - from.b) * t,
        a: from.a + (to.a - from.a) * t,
    }
}

/// Scales a colour's rgb toward black by `factor` (`1.0` = unchanged), alpha
/// kept.
pub const fn scale(color: Color, factor: f32) -> Color {
    Color {
        r: color.r * factor,
        g: color.g * factor,
        b: color.b * factor,
        a: color.a,
    }
}

/// Copies `color` with a replaced `alpha`.
pub const fn with_alpha(color: Color, alpha: f32) -> Color {
    Color {
        r: color.r,
        g: color.g,
        b: color.b,
        a: alpha,
    }
}

/// Scales `color`'s existing alpha by `factor`. Used by widgets that fade a
/// tinted surface in and out alongside a Hover animation.
pub const fn fade(color: Color, factor: f32) -> Color {
    with_alpha(color, color.a * factor)
}

/// Converts an opacity authored in sRGB space into the linear-space alpha the
/// wgpu renderer blends with. iced blends quads in linear space over an sRGB
/// surface, so a translucent white tint handed a raw CSS opacity blooms far
/// brighter than the design intends. Running the alpha through the sRGB
/// transfer keeps a white-over-dark glass tint as faint as the CSS value reads.
pub fn srgb_alpha(alpha: f32) -> f32 {
    if alpha <= 0.04045 {
        alpha / 12.92
    } else {
        ((alpha + 0.055) / 1.055).powf(2.4)
    }
}

/// Eases `base` toward `target` by `factor`, clamped to `[0, 1]`. Same as
/// [`mix`] but accepts unbounded factors from animation primitives without
/// requiring the caller to clamp.
pub fn ease(base: Color, target: Color, factor: f32) -> Color {
    mix(base, target, factor.clamp(0.0, 1.0))
}

/// Returns a configured iced theme for the current accent. Rebuilt each frame
/// by the app's `.theme(...)` closure so accent swaps land immediately.
pub fn theme() -> Theme {
    let palette = Palette {
        background: BG,
        text: TEXT_PRIMARY,
        primary: primary(),
        success: success(),
        warning: warning(),
        danger: error(),
    };

    Theme::Custom(Arc::new(Custom::new("Bluebottle".into(), palette)))
}
