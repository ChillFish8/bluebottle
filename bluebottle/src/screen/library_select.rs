use iced::{Element, task};

use crate::view;

#[derive(Default)]
pub struct LibrarySelectScreen {}

pub enum LibrarySelectMsg {}

impl view::View<LibrarySelectMsg> for LibrarySelectScreen {
    fn update(&mut self, _message: LibrarySelectMsg) -> task::Task<LibrarySelectMsg> {
        todo!()
    }

    fn view(&self) -> Element<'_, LibrarySelectMsg> {
        todo!()
    }
}
