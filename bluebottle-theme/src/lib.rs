use std::sync::Arc;
use iced::Color;
use iced::theme::{Custom, Palette};

mod dark;
mod utils;

pub fn bluebottle_base<A>(_app: &A) -> iced::Theme {
    iced::Theme::TokyoNightStorm
}


pub fn bluebottle_dark<A>(_app: &A) -> iced::Theme {
    iced::Theme::Custom(Arc::new(Custom::new(
        "BlueBottle Dark".into(),
        dark::PALETTE,
    )))
}

