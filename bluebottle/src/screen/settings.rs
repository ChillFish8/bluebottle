use iced::{Element, task};

use crate::view;

#[derive(Default)]
pub struct SettingsScreen {}

pub enum SettingsMsg {}

impl super::Screen<SettingsMsg> for SettingsScreen {
    fn nav_descriptor(&self) -> &str {
        "Settings"
    }
}

impl view::View<SettingsMsg> for SettingsScreen {
    fn update(&mut self, _message: SettingsMsg) -> task::Task<SettingsMsg> {
        task::Task::none()
    }

    fn view(&self) -> Element<'_, SettingsMsg> {
        iced::widget::column![].into()
    }
}
