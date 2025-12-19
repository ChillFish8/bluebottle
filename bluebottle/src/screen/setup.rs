use bluebottle_ui::{bar, color, font, separator, title};
use iced::widget::{column, container, row, space, text};
use iced::{Center, Element, Length};

use crate::view;

#[derive(Default)]
pub struct SetupScreen {
    jellyfin_server_url: String,
    jellyfin_username: String,
    jellyfin_password: String,
}

pub enum SetupMsg {}

impl view::View<SetupMsg> for SetupScreen {
    fn update(&mut self, _message: SetupMsg) {}

    fn view(&self) -> Element<'_, SetupMsg> {
        column![
            bar::top(space(), "Setup"),
            container(row![onboarding_menu()])
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

fn onboarding_menu<'a>() -> Element<'a, SetupMsg> {
    column![welcome(), add_library_message(),]
        .spacing(12)
        .into()
}

fn welcome() -> Element<'static, SetupMsg> {
    column![
        title::title(Some("waving_hand"), "Welcome to Bluebottle"),
        separator::seperator(Length::Fill).style(|theme| {
            separator::default_style(theme).background(color::PRIMARY)
        }),
    ]
    .spacing(4)
    .width(Length::Shrink)
    .into()
}

fn add_library_message() -> Element<'static, SetupMsg> {
    text("It looks like you haven't got any media libraries, so let's get that setup!")
        .font(font::regular())
        .size(18)
        .into()
}
