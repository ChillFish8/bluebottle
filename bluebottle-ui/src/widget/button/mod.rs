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
//! the 32px glass pill for controls floating over media.

pub use iced::widget::button::{Status, Style};

mod disabled;
mod hero;
mod icon;
mod nav;
mod pill;
mod standard;

pub use disabled::disabled;
pub use hero::hero;
pub use icon::{IconSizeVariant, icon, icon_carousel, icon_flat, icon_overlay};
pub use nav::nav;
pub use pill::{ghost, ghost_small, toggle_pill};
pub use standard::standard;
