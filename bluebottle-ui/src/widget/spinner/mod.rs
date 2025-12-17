//! Based on the iced loading_spinners example but stripped out the theming.

use iced::Element;

mod circular;
mod linear;

/// A linear bar spinner.
pub fn linear<'a>() -> linear::Linear<'a> {
    linear::Linear::new()
}

/// A circle spinner.
pub fn circle<'a>() -> circular::Circular<'a> {
    circular::Circular::new()
}
