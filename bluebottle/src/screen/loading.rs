use bluebottle_ui::{bar, button, spinner, text};
use iced::widget::{column, container, row, space};
use iced::{Center, Element, Length, padding, task};

use crate::{navigator, view};

#[derive(Default)]
pub struct LoadingScreen {}

#[derive(Clone)]
pub enum LoadingMsg {
    NavigateLibrarySelect,
    NavigateSettings,
}

impl super::Screen<LoadingMsg> for LoadingScreen {
    fn nav_descriptor(&self) -> &str {
        "Loading Library"
    }
}

impl view::View<LoadingMsg> for LoadingScreen {
    fn update(&mut self, message: LoadingMsg) -> task::Task<LoadingMsg> {
        match message {
            LoadingMsg::NavigateLibrarySelect => {
                navigator::navigate(navigator::ActiveScreen::LibrarySelect);
            },
            LoadingMsg::NavigateSettings => {
                navigator::navigate(navigator::ActiveScreen::Settings);
            },
        };

        task::Task::none()
    }

    fn view(&self) -> Element<'_, LoadingMsg> {
        column![
            row![
                bar::side(
                    space(),
                    column![
                        button::nav(
                            "Library",
                            "apps",
                            false,
                            LoadingMsg::NavigateLibrarySelect
                        ),
                        button::nav(
                            "Settings",
                            "settings",
                            false,
                            LoadingMsg::NavigateSettings
                        ),
                    ]
                    .spacing(4)
                ),
                container(
                    column![
                        text::title(None, "Getting your library setup"),
                        text::paragraph("This will only take a few moments..."),
                        spinner::linear(),
                    ]
                    .spacing(16)
                    .align_x(Center)
                    .width(Length::Shrink)
                )
                .width(Length::Fill)
                .align_x(Center)
                .padding(padding::right(64)),
            ]
            .height(Length::Fill)
            .width(Length::Fill)
            .align_y(Center),
        ]
        .align_x(Center)
        .into()
    }
}
