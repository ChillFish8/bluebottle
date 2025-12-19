use iced::Element;

use super::Screen;

#[derive(Default)]
pub struct SettingsScreen {}

pub enum SettingsMsg {}

impl Screen<SettingsMsg> for SettingsScreen {
    fn update(&mut self, message: SettingsMsg) {
        todo!()
    }

    fn view(&self) -> Element<'_, SettingsMsg> {
        todo!()
    }
}
