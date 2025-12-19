use iced::Element;

use crate::view;

#[derive(Default)]
pub struct LibrarySelectScreen {}

pub enum LibrarySelectMsg {}

impl view::View<LibrarySelectMsg> for LibrarySelectScreen {
    fn update(&mut self, message: LibrarySelectMsg) {
        todo!()
    }

    fn view(&self) -> Element<'_, LibrarySelectMsg> {
        todo!()
    }
}
