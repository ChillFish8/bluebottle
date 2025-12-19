use iced::Element;

use super::Screen;

#[derive(Default)]
pub struct LibrarySelectScreen {}

pub enum LibrarySelectMsg {}

impl Screen<LibrarySelectMsg> for LibrarySelectScreen {
    fn update(&mut self, message: LibrarySelectMsg) {
        todo!()
    }

    fn view(&self) -> Element<'_, LibrarySelectMsg> {
        todo!()
    }
}
