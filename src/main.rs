use iced::widget::{center, column, row, slider, text, Column};
use iced::{Center, Element};

use std::time::Duration;

pub fn main() -> iced::Result {
    iced::application(
        BlueBottle::default,
        BlueBottle::update,
        BlueBottle::view,
    )
    .title("BlueBottle Media Player")
    .run()
}


struct BlueBottle {

}

impl BlueBottle {
    fn view(&self) -> Element<'_, Message> {
        column![].into()
    }

    fn update(&mut self, message: Message) {

    }
}

impl Default for BlueBottle {
    fn default() -> Self {
        Self { }
    }
}

enum Message {

}