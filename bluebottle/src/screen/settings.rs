use iced::Element;

use crate::view;

#[derive(Default)]
pub struct SettingsScreen {}

pub enum SettingsMsg {}

impl view::View<SettingsMsg> for SettingsScreen {
    fn update(&mut self, message: SettingsMsg) {
        todo!()
    }

    fn view(&self) -> Element<'_, SettingsMsg> {
        todo!()
    }
}
