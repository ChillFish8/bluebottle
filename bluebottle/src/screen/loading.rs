use iced::Element;

use super::Screen;

#[derive(Default)]
pub struct LoadingScreen {}

pub enum LoadingMsg {}

impl Screen<LoadingMsg> for LoadingScreen {
    fn update(&mut self, message: LoadingMsg) {
        todo!()
    }

    fn view(&self) -> Element<'_, LoadingMsg> {
        todo!()
    }
}
