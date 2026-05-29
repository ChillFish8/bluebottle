//! Button widgets. Hover and press are eased by the design system's 100 ms
//! [`Hover`](crate::animate::hover::Hover) primitive rather than iced's
//! instant status-based styling. Hovering fades a pill tint in behind the
//! content. Pressing eases the text and icon colour toward `primary()` (or
//! away from it for a selected `toggle_icon`). `standard`, `icon`, and
//! `toggle_icon` are thin builders over
//! [`clickable`](super::clickable::clickable). `nav` is its own custom
//! widget because its `selected` state has a second animation track that
//! cross-fades the pill behind the icon. `disabled` stays a plain iced
//! `button` since it has nothing to animate.

pub use iced::widget::button::{Status, Style};

mod disabled;
mod icon;
mod nav;
mod standard;
mod toggle_icon;

pub use disabled::disabled;
pub use icon::{IconTextOrName, icon};
pub use nav::nav;
pub use standard::standard;
pub use toggle_icon::toggle_icon;

/// Padding shared by the icon button and the disabled button so a disabled
/// icon slots into the same rows without shifting.
const ICON_PADDING: u16 = 4;
