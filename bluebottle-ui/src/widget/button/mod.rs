//! Button widgets. Hover and press are eased by the design system's 100 ms
//! [`Hover`](crate::animate::hover::Hover) primitive rather than iced's
//! instant status-based styling. Hovering fades a pill tint in behind the
//! content. Pressing eases the text and icon colour toward `primary()`.
//! `standard`, `icon`, and `icon_flat` are thin builders over
//! [`clickable`](super::clickable::clickable). `nav` is its own custom
//! widget because its `selected` state has a second animation track that
//! cross-fades the pill behind the icon. `disabled` stays a plain iced
//! `button` since it has nothing to animate.
//!
//! `icon` is the bordered glass circle, a white glass fill behind a hairline
//! that turns accent when on. `icon_flat` is the border-free variant for
//! denser rows. `icon_carousel` is the 26px paging chevron. `icon_overlay` is
//! the 32px glass pill for controls floating over media. `dismiss` and
//! `dismiss_icon` are the two close affordances, a labelled pill for the
//! player bar and a 28px circle for the ambient header. `checkbox` is the
//! bordered glass box that swaps to the accent recipe and strokes in the
//! animated check when on.

pub use iced::widget::button::{Status, Style};

mod chassis;
mod checkbox;
mod disabled;
mod hero;
mod icon;
mod media;
mod nav;
mod pill;
mod standard;
mod switch;
mod utility;

pub use checkbox::{CheckboxSizeVariant, checkbox};
pub use disabled::disabled;
pub use hero::hero;
pub use icon::{
    ICON_FLAT_DIAMETER,
    ICON_FLAT_GLYPH,
    IconFlatButton,
    IconSizeVariant,
    icon,
    icon_carousel,
    icon_flat,
    icon_overlay,
};
pub use media::{
    AccentSizeVariant,
    PrimarySizeVariant,
    accent,
    mode,
    primary,
    skip,
    transport_mini,
};
pub use nav::nav;
pub use pill::{ghost, ghost_small, toggle_pill};
pub use standard::standard;
pub use switch::{Switch, SwitchSizeVariant, switch};
pub use utility::{dismiss, dismiss_icon};
