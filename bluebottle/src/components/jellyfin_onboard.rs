//! Onboard a new Jellyfin media library.

use bluebottle_ui::{button, separator};
use iced::widget::{column, row};
use iced::{Center, Element, Length, padding};

use crate::view;

#[derive(Default)]
pub struct JellyfinOnboard {
    jellyfin_server_url: String,
    jellyfin_username: String,
    jellyfin_password: String,
    parsed_jellyfin_server_url: Option<url::Url>,
    stage: Stage,
    test_failed: bool,
}

#[derive(Clone)]
pub enum JellyfinOnboardMsg {
    NavigateServer,
    NavigateUser,
    NavigateTest,
    NavigateCustomise,
    ServerUrl(String),
    Username(String),
    Password(String),
    SubmitUrl,
    SubmitUser,
    RetryTest,
    TestSuccess,
    TestFail,
}

impl view::View<JellyfinOnboardMsg> for JellyfinOnboard {
    fn update(&mut self, message: JellyfinOnboardMsg) {
        match message {
            JellyfinOnboardMsg::NavigateServer => {
                self.navigate(Stage::AddServer);
            },
            JellyfinOnboardMsg::NavigateUser => {
                self.navigate(Stage::AddUser);
            },
            JellyfinOnboardMsg::NavigateTest => {
                self.navigate(Stage::Test);
            },
            JellyfinOnboardMsg::NavigateCustomise => {
                self.navigate(Stage::Customise);
            },
            JellyfinOnboardMsg::ServerUrl(value) => {
                self.parsed_jellyfin_server_url = url::Url::parse(&value).ok();
                self.jellyfin_server_url = value;
            },
            JellyfinOnboardMsg::Username(value) => {
                self.jellyfin_username = value;
            },
            JellyfinOnboardMsg::Password(value) => {
                self.jellyfin_password = value;
            },
            JellyfinOnboardMsg::SubmitUrl => {
                if self.is_url_valid() {
                    self.stage = Stage::AddUser;
                    self.test_failed = false;
                }
            },
            JellyfinOnboardMsg::SubmitUser => {
                if self.is_user_valid() {
                    self.stage = Stage::Test;
                    self.test_failed = false;
                }
            },
            JellyfinOnboardMsg::TestSuccess => {
                self.stage = Stage::Customise;
                self.test_failed = false;
            },
            JellyfinOnboardMsg::TestFail => {
                self.test_failed = true;
            },
            JellyfinOnboardMsg::RetryTest => {
                self.test_failed = false;
            },
        }
    }

    fn view(&self) -> Element<'_, JellyfinOnboardMsg> {
        column![self.navbar()]
            .spacing(16)
            .padding(padding::vertical(16))
            .width(Length::Fill)
            .into()
    }
}

impl JellyfinOnboard {
    fn navbar(&self) -> Element<'_, JellyfinOnboardMsg> {
        row![
            button::standard(
                "Server",
                Some("storage"),
                false,
                JellyfinOnboardMsg::NavigateServer
            ),
            separator::seperator(Length::Fill).style(separator::primary_style),
            button::standard(
                "User",
                Some("account_box"),
                false,
                JellyfinOnboardMsg::NavigateUser
            ),
            separator::seperator(Length::Fill).style(separator::primary_style),
            button::standard(
                "Test",
                Some("network_check"),
                false,
                JellyfinOnboardMsg::NavigateTest
            ),
            separator::seperator(Length::Fill).style(separator::primary_style),
            button::standard(
                "Customise",
                Some("dashboard_customize"),
                false,
                JellyfinOnboardMsg::NavigateCustomise
            ),
            separator::seperator(Length::Fill).style(separator::primary_style),
            button::standard(
                "Complete",
                Some("done_all"),
                false,
                JellyfinOnboardMsg::NavigateCustomise
            ),
        ]
        .align_y(Center)
        .spacing(4)
        .into()
    }

    fn navigate(&mut self, stage: Stage) {
        tracing::debug!(stage = ?stage, "navigate");
        self.stage = stage;
    }

    /// Returns whether the provided server URL is valid or not.
    fn is_url_valid(&self) -> bool {
        self.parsed_jellyfin_server_url.is_some()
    }

    /// Returns whether the specified user and password is valid (to submit) or not.
    fn is_user_valid(&self) -> bool {
        !self.jellyfin_username.is_empty() && !self.jellyfin_password.is_empty()
    }

    /// Returns the parsed Jellyfin server URL.
    ///
    /// Panics if the URL is invalid.
    fn parsed_url(&self) -> &url::Url {
        self.parsed_jellyfin_server_url.as_ref().unwrap()
    }
}

#[derive(Debug, Default)]
enum Stage {
    #[default]
    AddServer,
    AddUser,
    Test,
    Customise,
    Complete,
}
