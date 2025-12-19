use bluebottle_ui::bar;
use iced::widget::{container, row, space};
use iced::{Element, Length, task};

use crate::view;

#[derive(Default)]
pub struct LibraryViewScreen {}

pub enum LibraryViewMsg {}

impl view::View<LibraryViewMsg> for LibraryViewScreen {
    fn update(&mut self, _message: LibraryViewMsg) -> task::Task<LibraryViewMsg> {
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
