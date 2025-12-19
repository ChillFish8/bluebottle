use iced::Element;

use crate::view;

#[derive(Default)]
pub struct LoadingScreen {}

pub enum LoadingMsg {}

impl view::View<LoadingMsg> for LoadingScreen {
    fn update(&mut self, message: LoadingMsg) {
        todo!()
    }

    fn view(&self) -> Element<'_, LoadingMsg> {
        todo!()
    }
}
