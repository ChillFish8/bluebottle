use bluebottle_ui::text;
use iced::widget::{column, container, row};
use iced::{Center, Element, Length, padding, task};

use crate::components::jellyfin_onboard::{JellyfinOnboard, JellyfinOnboardMsg};
use crate::view;

#[derive(Default)]
pub struct SetupScreen {
    jellyfin_onboard: JellyfinOnboard,
}

#[derive(Clone)]
pub enum SetupMsg {
    JellyfinOnboard(JellyfinOnboardMsg),
}

impl super::Screen<SetupMsg> for SetupScreen {
    const HIDE_SIDEBAR: bool = true;

    fn nav_descriptor(&self) -> &str {
        "Setup"
    }
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

        let message = column![
            text::title(Some("waving_hand"), "Welcome to Bluebottle"),
            text::subheading(
                "It looks like you haven't got any media libraries. Let's add one!"
            ),
        ]
        .spacing(8)
        .padding(padding::horizontal(8));

        column![
            message,
            self.jellyfin_onboard.view().map(SetupMsg::JellyfinOnboard),
        ]
        .width(1000)
        .spacing(16)
        .into()
    }
}
