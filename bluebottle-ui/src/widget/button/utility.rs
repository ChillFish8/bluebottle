use iced::widget::{container, row};
use iced::{Center, Element, padding};

use super::chassis::icon_circle;
use crate::widget::clickable::clickable;
use crate::{color, font, icon, spacing, text};

/// Close / Dismiss · Pill
///
/// The labelled dismiss for the player-mode bar. A glass pill at 10% white
/// behind a 10% hairline carrying a × glyph and a "Close" label. Hover lifts
/// the fill to roughly double its rest weight.
pub fn dismiss<'a, Message>(message: Message) -> Element<'a, Message>
where
    Message: Clone + 'a,
{
    let glyph = icon::filled("close").size(14);
    let label = text::label("Close", text::Variant::Main).font(font::semibold());
    let optically_aligned = container(label).padding(padding::bottom(1));

    // Small diversion from the design spec, this is because we're adjusting for the font and
    // icon sizes to get the "optically centered" look, which is why we shifted the whole
    // item line down 1px and offset the label by +1 px relative to the icon.
    let items = row![glyph, optically_aligned]
        .spacing(spacing::GAP_4)
        .align_y(Center);
    // 14/8/9 px lift the dismiss pill's label off-centre to keep the icon
    // and label optically aligned. Kept as literals.
    let pad = padding::Padding::default().horizontal(14).top(8).bottom(9);
    let glass = color::border_strong();

    clickable(items)
        .padding(pad)
        .background(glass)
        .tint(glass)
        .border(glass)
        .on_press(message)
        .into()
}

/// Close / Dismiss · Circle
///
/// The bare dismiss for the ambient-mode header. A 28px circle carrying a ×
/// glyph, resting on 6% white behind a 10% hairline. Hover lifts the fill to
/// match the pill's hover weight.
pub fn dismiss_icon<'a, Message>(message: Message) -> Element<'a, Message>
where
    Message: Clone + 'a,
{
    clickable(icon_circle("close", 28.0, 14.0))
        .background(color::border())
        .tint(color::border_strong())
        .border(color::border_strong())
        .on_press(message)
        .into()
}
