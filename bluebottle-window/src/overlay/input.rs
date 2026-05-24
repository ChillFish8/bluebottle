//! Translation from Wayland seat events into Iced input events.
//!
//! This mirrors the relevant parts of `iced_winit::conversion`, adapted to the
//! `smithay-client-toolkit` event types since we drive the seat directly.

use iced_runtime::core::keyboard::key::{Key, Named, NativeCode, Physical};
use iced_runtime::core::{keyboard, mouse};
use smithay_client_toolkit::seat::keyboard::{Keysym, Modifiers as SctkModifiers};

/// Linux button codes from `<linux/input-event-codes.h>`.
const BTN_LEFT: u32 = 0x110;
const BTN_RIGHT: u32 = 0x111;
const BTN_MIDDLE: u32 = 0x112;
const BTN_SIDE: u32 = 0x113;
const BTN_EXTRA: u32 = 0x114;

/// Map a Wayland pointer button code to an Iced [`mouse::Button`].
pub(crate) fn mouse_button(code: u32) -> mouse::Button {
    match code {
        BTN_LEFT => mouse::Button::Left,
        BTN_RIGHT => mouse::Button::Right,
        BTN_MIDDLE => mouse::Button::Middle,
        BTN_SIDE => mouse::Button::Back,
        BTN_EXTRA => mouse::Button::Forward,
        other => mouse::Button::Other((other & 0xFFFF) as u16),
    }
}

/// Convert `smithay-client-toolkit` modifier flags to Iced [`keyboard::Modifiers`].
pub(crate) fn modifiers(mods: SctkModifiers) -> keyboard::Modifiers {
    let mut result = keyboard::Modifiers::empty();
    result.set(keyboard::Modifiers::CTRL, mods.ctrl);
    result.set(keyboard::Modifiers::ALT, mods.alt);
    result.set(keyboard::Modifiers::SHIFT, mods.shift);
    result.set(keyboard::Modifiers::LOGO, mods.logo);
    result
}

/// Build a key-press or key-release Iced [`keyboard::Event`].
pub(crate) fn key_event(
    keysym: Keysym,
    utf8: Option<String>,
    modifiers: keyboard::Modifiers,
    pressed: bool,
    repeat: bool,
) -> keyboard::Event {
    let key = key(keysym, utf8.as_deref());
    let physical_key = Physical::Unidentified(NativeCode::Unidentified);

    if pressed {
        let text = utf8.filter(|t| !t.is_empty() && !is_control(t));
        keyboard::Event::KeyPressed {
            key: key.clone(),
            modified_key: key,
            physical_key,
            location: keyboard::Location::Standard,
            modifiers,
            text: text.map(Into::into),
            repeat,
        }
    } else {
        keyboard::Event::KeyReleased {
            key: key.clone(),
            modified_key: key,
            physical_key,
            location: keyboard::Location::Standard,
            modifiers,
        }
    }
}

/// Map an xkb keysym (plus any produced text) to an Iced [`Key`].
fn key(keysym: Keysym, utf8: Option<&str>) -> Key {
    if let Some(named) = named_key(keysym) {
        return Key::Named(named);
    }

    match utf8 {
        Some(text) if !text.is_empty() && !is_control(text) => {
            Key::Character(text.into())
        },
        _ => Key::Unidentified,
    }
}

/// Map the common named xkb keysyms to Iced [`Named`] keys.
fn named_key(keysym: Keysym) -> Option<Named> {
    Some(match keysym {
        Keysym::Return | Keysym::KP_Enter => Named::Enter,
        Keysym::BackSpace => Named::Backspace,
        Keysym::Delete => Named::Delete,
        Keysym::Tab => Named::Tab,
        Keysym::Escape => Named::Escape,
        Keysym::space => Named::Space,
        Keysym::Left => Named::ArrowLeft,
        Keysym::Right => Named::ArrowRight,
        Keysym::Up => Named::ArrowUp,
        Keysym::Down => Named::ArrowDown,
        Keysym::Home => Named::Home,
        Keysym::End => Named::End,
        Keysym::Page_Up => Named::PageUp,
        Keysym::Page_Down => Named::PageDown,
        _ => return None,
    })
}

/// Whether the text is entirely control characters (and so not insertable).
fn is_control(text: &str) -> bool {
    text.chars().all(char::is_control)
}
