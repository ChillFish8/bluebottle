use bluebottle_ui::bar;
use iced::widget::space;
use iced::{Element, task};

use crate::view;

#[derive(Default)]
pub struct LibrarySelectScreen {}

pub enum LibrarySelectMsg {}

impl view::View<LibrarySelectMsg> for LibrarySelectScreen {
    fn update(&mut self, _message: LibrarySelectMsg) -> task::Task<LibrarySelectMsg> {
        task::Task::none()
    }

    fn view(&self) -> Element<'_, LibrarySelectMsg> {
        iced::widget::column![bar::top(space(), "Library Select"),].into()
    }
}
