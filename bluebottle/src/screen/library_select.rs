use iced::{Element, task};

use crate::view;

#[derive(Default)]
pub struct LibrarySelectScreen {}

pub enum LibrarySelectMsg {}

impl super::Screen<LibrarySelectMsg> for LibrarySelectScreen {
    fn nav_descriptor(&self) -> &str {
        "Library Select"
    }
}

impl view::View<LibrarySelectMsg> for LibrarySelectScreen {
    fn update(&mut self, _message: LibrarySelectMsg) -> task::Task<LibrarySelectMsg> {
        task::Task::none()
    }

    fn view(&self) -> Element<'_, LibrarySelectMsg> {
        iced::widget::column![].into()
    }
}
