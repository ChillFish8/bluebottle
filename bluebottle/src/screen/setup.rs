use bluebottle_ui::{bar, color, font, separator, title};
use iced::widget::{column, container, row, space, text};
use iced::{Center, Element, Length, task};

use crate::components::jellyfin_onboard::{JellyfinOnboard, JellyfinOnboardMsg};
use crate::view;

#[derive(Default)]
pub struct SetupScreen {
    jellyfin_onboard: JellyfinOnboard,
}

pub enum SetupMsg {
    JellyfinOnboard(JellyfinOnboardMsg),
}

impl view::View<SetupMsg> for SetupScreen {
    fn update(&mut self, message: SetupMsg) -> task::Task<SetupMsg> {
        match message {
            SetupMsg::JellyfinOnboard(msg) => self
                .jellyfin_onboard
                .update(msg)
                .map(SetupMsg::JellyfinOnboard),
        }
    }

    fn view(&self) -> Element<'_, SetupMsg> {
        column![
            bar::top(space(), "Setup"),
            container(row![self.onboarding_menu()])
                .width(Length::Fill)
                .height(Length::Fill)
                .align_x(Center)
                .align_y(Center),
        ]
        .height(Length::Fill)
        .align_x(Center)
        .into()
    }
}

impl SetupScreen {
    fn onboarding_menu(&self) -> Element<'_, SetupMsg> {
        use view::View;

        column![
            title::title(Some("waving_hand"), "Welcome to Bluebottle"),
            add_library_message(),
            self.jellyfin_onboard.view().map(SetupMsg::JellyfinOnboard),
        ]
        .width(1000)
        .spacing(12)
        .align_x(Center)
        .into()
    }
}

fn add_library_message() -> Element<'static, SetupMsg> {
    text("It looks like you haven't got any media libraries. Let's add one!")
        .font(font::regular())
        .size(18)
        .into()
}
