use iced::Color;

use crate::color;

/// Bead colour family. Beads sit on glass surfaces in [`Tone::Accent`] and on
/// solid surfaces in [`Tone::White`].
#[derive(Clone, Copy, Default, PartialEq, Eq, Debug)]
pub enum Tone {
    #[default]
    Accent,
    White,
}

const ACCENT_FILL: f32 = 0.30;
const ACCENT_RIM: f32 = 0.85;
const WHITE_FILL: f32 = 0.20;
const WHITE_RIM: f32 = 0.45;

/// Glass fill for a bead of `tone`.
pub fn fill(tone: Tone) -> Color {
    match tone {
        Tone::Accent => {
            color::with_alpha(color::primary(), color::srgb_alpha(ACCENT_FILL))
        },
        Tone::White => color::with_alpha(color::WHITE, color::srgb_alpha(WHITE_FILL)),
    }
}

/// Hairline rim for a bead of `tone`.
pub fn rim(tone: Tone) -> Color {
    match tone {
        Tone::Accent => {
            color::with_alpha(color::primary(), color::srgb_alpha(ACCENT_RIM))
        },
        Tone::White => color::with_alpha(color::WHITE, color::srgb_alpha(WHITE_RIM)),
    }
}
