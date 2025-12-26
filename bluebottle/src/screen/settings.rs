use bluebottle_ui::bar;
use iced::widget::space;
use iced::{Element, task};

use crate::view;

#[derive(Default)]
pub struct SettingsScreen {}

pub enum SettingsMsg {}

impl view::View<SettingsMsg> for SettingsScreen {
    fn update(&mut self, _message: SettingsMsg) -> task::Task<SettingsMsg> {
        task::Task::none()
    }

    fn view(&self) -> Element<'_, SettingsMsg> {
        iced::widget::column![bar::top(space(), "Settings"),].into()
    }
}
