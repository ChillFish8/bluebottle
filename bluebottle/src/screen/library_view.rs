use bluebottle_ui::bar;
use iced::widget::{container, row, space};
use iced::{Element, Length};

use super::Screen;

#[derive(Default)]
pub struct LibraryViewScreen {}

pub enum LibraryViewMsg {}

impl Screen<LibraryViewMsg> for LibraryViewScreen {
    fn update(&mut self, message: LibraryViewMsg) {
        todo!()
    }

    fn view(&self) -> Element<'_, LibraryViewMsg> {
        iced::widget::column![
            bar::top(space(), "Example Library"),
            row![
                bar::side(space(), space()),
                container(space()).width(Length::Fill).height(Length::Fill)
            ]
        ]
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
    }
}
