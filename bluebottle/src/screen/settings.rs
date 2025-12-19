use iced::{Element, task};

use crate::view;

#[derive(Default)]
pub struct SettingsScreen {}

pub enum SettingsMsg {}

impl view::View<SettingsMsg> for SettingsScreen {
    fn update(&mut self, _message: SettingsMsg) -> task::Task<SettingsMsg> {
        todo!()
    }

    fn view(&self) -> Element<'_, SettingsMsg> {
        todo!()
    }
}
