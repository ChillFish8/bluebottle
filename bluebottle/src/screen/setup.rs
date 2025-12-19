use bluebottle_ui::bar;
use iced::Element;
use iced::widget::{column, row, space};

use super::Screen;

#[derive(Default)]
pub struct SetupScreen {}

pub enum SetupMsg {}

impl Screen<SetupMsg> for SetupScreen {
    fn update(&mut self, _message: SetupMsg) {}

    fn view(&self) -> Element<'_, SetupMsg> {
        column![bar::top(space(), ""), row![]].into()
    }
}
