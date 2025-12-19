//! Onboard a new Jellyfin media library.

use bluebottle_ui::{button, color, font, input, separator};
use iced::widget::{column, container, row, space, text};
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
    test_completed: bool,
}

#[derive(Clone)]
pub enum JellyfinOnboardMsg {
    Nop,
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
            JellyfinOnboardMsg::Nop => {},
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
        let subsection = match self.stage {
            Stage::AddServer => self.server_setup(),
            Stage::AddUser => self.user_setup(),
            Stage::Test => space().into(),
            Stage::Customise => space().into(),
            Stage::Complete => space().into(),
        };

        column![self.navbar(), subsection]
            .spacing(16)
            .padding(padding::vertical(16))
            .width(Length::Fill)
            .height(500)
            .into()
    }
}

impl JellyfinOnboard {
    fn navigate(&mut self, stage: Stage) {
        tracing::debug!(stage = ?stage, "navigate");
        self.stage = stage;
    }

    /// Returns whether the provided server URL is valid or not.
    fn is_url_valid(&self) -> bool {
        if let Some(url) = self.parsed_jellyfin_server_url.as_ref() {
            matches!(url.scheme(), "http" | "https")
        } else {
            false
        }
    }

    /// Returns whether the specified user and password is valid (to submit) or not.
    fn is_user_valid(&self) -> bool {
        !self.jellyfin_username.is_empty() && !self.jellyfin_password.is_empty()
    }

    /// Returns if the test is complete and it was successful.
    fn test_completed_successfully(&self) -> bool {
        self.test_completed && !self.test_failed
    }

    /// Returns whether the onboarding has been completed.
    fn is_complete(&self) -> bool {
        self.is_url_valid() && self.is_user_valid() && self.test_completed_successfully()
    }

    /// Returns the parsed Jellyfin server URL.
    ///
    /// Panics if the URL is invalid.
    fn parsed_url(&self) -> &url::Url {
        self.parsed_jellyfin_server_url.as_ref().unwrap()
    }

    fn navbar(&self) -> Element<'_, JellyfinOnboardMsg> {
        row![
            nav_button(
                "Server",
                "storage",
                JellyfinOnboardMsg::NavigateServer,
                self.stage == Stage::AddServer,
                false
            ),
            connector_line(!self.is_url_valid()),
            nav_button(
                "User",
                "account_box",
                JellyfinOnboardMsg::NavigateUser,
                self.stage == Stage::AddUser,
                !self.is_url_valid()
            ),
            connector_line(!self.is_user_valid()),
            nav_button(
                "Test",
                "network_check",
                JellyfinOnboardMsg::NavigateTest,
                self.stage == Stage::Test,
                !self.is_user_valid()
            ),
            connector_line(!self.test_completed_successfully()),
            nav_button(
                "Customise",
                "dashboard_customize",
                JellyfinOnboardMsg::NavigateCustomise,
                self.stage == Stage::Customise,
                !self.test_completed_successfully()
            ),
            connector_line(!self.is_complete()),
            nav_button(
                "Complete",
                "done_all",
                JellyfinOnboardMsg::Nop,
                self.stage == Stage::Complete,
                !self.is_complete()
            ),
        ]
        .align_y(Center)
        .spacing(4)
        .into()
    }

    fn server_setup(&self) -> Element<'_, JellyfinOnboardMsg> {
        column![
            form_label("Server Address"),
            input::text_input(
                "Server URL...",
                &self.jellyfin_server_url,
                JellyfinOnboardMsg::ServerUrl,
            )
        ]
        .spacing(4)
        .into()
    }

    fn user_setup(&self) -> Element<'_, JellyfinOnboardMsg> {
        column![
            column![
                form_label("Username"),
                input::text_input(
                    "Username...",
                    &self.jellyfin_username,
                    JellyfinOnboardMsg::Username,
                ),
            ]
            .spacing(4),
            column![
                form_label("Password"),
                input::text_input(
                    "Super secure password...",
                    &self.jellyfin_password,
                    JellyfinOnboardMsg::Password,
                )
                .secure(true),
            ]
            .spacing(4),
        ]
        .spacing(16)
        .into()
    }
}

#[derive(Debug, Default, Eq, PartialEq)]
enum Stage {
    #[default]
    AddServer,
    AddUser,
    Test,
    Customise,
    Complete,
}

fn nav_button<'a>(
    label: &'a str,
    icon: &'a str,
    message: JellyfinOnboardMsg,
    selected: bool,
    disabled: bool,
) -> Element<'a, JellyfinOnboardMsg> {
    if disabled {
        button::disabled(Some(label), Some(icon))
    } else {
        button::standard(label, Some(icon), selected, message).into()
    }
}

fn connector_line<'a>(disabled: bool) -> Element<'a, JellyfinOnboardMsg> {
    let mut seperator = separator::seperator(Length::Fill);
    if !disabled {
        seperator = seperator.style(separator::primary_style);
    }
    seperator.into()
}

fn form_label(label: &str) -> Element<'_, JellyfinOnboardMsg> {
    let label = text(label)
        .size(12)
        .font(font::semibold())
        .color(color::TEXT_SECONDARY);
    container(label).padding(padding::horizontal(16)).into()
}
