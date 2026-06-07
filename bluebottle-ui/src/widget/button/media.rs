use iced::{Color, Element};

use super::chassis::icon_circle;
use crate::color;
use crate::widget::clickable::clickable;

/// Sizes for [`primary`]. Mini dock at 36, the video player at 52, the full
/// Ambient player at 64.
#[derive(Debug, Eq, PartialEq, Copy, Clone, Default)]
pub enum PrimarySizeVariant {
    Small,
    #[default]
    Medium,
    Large,
}

/// Primary · Play / Pause
///
/// The transport's anchor and the only solid control in the cluster. A pure
/// white disc with a dark glyph at roughly 40% of the diameter behind a soft
/// accent glow. Hover deepens the glow without lifting a tint over the white,
/// the same recipe as the hero button.
pub fn primary<'a, Message>(
    icon_name: &'a str,
    size: PrimarySizeVariant,
    message: Message,
) -> Element<'a, Message>
where
    Message: Clone + 'a,
{
    let (diameter, icon_size) = match size {
        PrimarySizeVariant::Small => (36.0, 14.0),
        PrimarySizeVariant::Medium => (52.0, 20.0),
        PrimarySizeVariant::Large => (64.0, 26.0),
    };

    clickable(icon_circle(icon_name, diameter, icon_size))
        .background(color::WHITE)
        .glow()
        .resting_color(color::BG)
        .tint(Color::TRANSPARENT)
        .on_press(message)
        .into()
}

/// Transport · Skip
///
/// The 44px previous and next buttons flanking the play button. A glass 8%
/// fill that lifts to roughly 16% on hover with a white glyph at full
/// strength.
pub fn skip<'a, Message>(icon_name: &'a str, message: Message) -> Element<'a, Message>
where
    Message: Clone + 'a,
{
    let glass = color::with_alpha(color::WHITE, color::srgb_alpha(0.08));
    clickable(icon_circle(icon_name, 44.0, 18.0))
        .background(glass)
        .tint(glass)
        .resting_color(color::TEXT_PRIMARY)
        .on_press(message)
        .into()
}

/// Transport · Mode
///
/// The 38px shuffle, repeat, queue toggles in the transport cluster. Shares
/// the [`skip`] glass recipe but eases the glyph between text-secondary while
/// off and white when engaged so the engaged mode reads at a glance and the
/// transition tracks the rest of the design system's 100 ms ease.
pub fn mode<'a, Message>(
    icon_name: &'a str,
    on: bool,
    message: Message,
) -> Element<'a, Message>
where
    Message: Clone + 'a,
{
    let glass = color::with_alpha(color::WHITE, color::srgb_alpha(0.08));
    clickable(icon_circle(icon_name, 38.0, 16.0))
        .background(glass)
        .tint(glass)
        .resting_color(color::TEXT_SECONDARY)
        .selected(on)
        .selected_color(color::TEXT_PRIMARY)
        .on_press(message)
        .into()
}

/// Transport · Mini
///
/// The 28px skip control for the tight mini dock. Transparent at rest, hover
/// paints the neutral [`color::HOVER`] fill rather than a translucent veil.
pub fn transport_mini<'a, Message>(
    icon_name: &'a str,
    message: Message,
) -> Element<'a, Message>
where
    Message: Clone + 'a,
{
    // The default clickable tint is color::HOVER, which already matches the
    // mini-dock spec, so the chain stays minimal.
    clickable(icon_circle(icon_name, 28.0, 14.0))
        .on_press(message)
        .into()
}

/// Sizes for [`accent`]. Main is the 48px default; Alt is the 56px variant for
/// larger poster tiles.
#[derive(Debug, Eq, PartialEq, Copy, Clone, Default)]
pub enum AccentSizeVariant {
    #[default]
    Main,
    Alt,
}

impl AccentSizeVariant {
    /// Outer diameter in logical pixels. Callers sizing a backdrop
    /// concentric with the button should pull from here.
    pub const fn diameter(self) -> f32 {
        match self {
            Self::Main => 48.0,
            Self::Alt => 56.0,
        }
    }

    /// Inner glyph size in logical pixels.
    pub const fn glyph(self) -> f32 {
        match self {
            Self::Main => 18.0,
            Self::Alt => 20.0,
        }
    }
}

/// Accent · Hover-Reveal Play
///
/// The play affordance that lives on a content tile and fades in once the card
/// is hovered. An accent 28% fill behind a full accent ring. Hovering the disc
/// itself lifts the fill toward 55% so the indigo turns near-solid. The reveal
/// fade is the surface's job. The 14px backdrop blur called for by the spec is
/// also the surface's responsibility. iced does not composite a backdrop pass.
pub fn accent<'a, Message>(
    icon_name: &'a str,
    size: AccentSizeVariant,
    message: Message,
) -> Element<'a, Message>
where
    Message: Clone + 'a,
{
    let diameter = size.diameter();
    let icon_size = size.glyph();

    // The hover tint stacks on the resting 28% accent fill. Linear-space
    // compositing means a thin tint barely lifts the perceived fill, so the
    // tint is authored well above the perceptual delta to land the combined
    // coverage near the spec's 55%.
    clickable(icon_circle(icon_name, diameter, icon_size))
        .background(color::primary_glass())
        .tint(color::with_alpha(color::primary(), color::srgb_alpha(0.45)))
        .border(color::primary())
        .on_press(message)
        .into()
}
