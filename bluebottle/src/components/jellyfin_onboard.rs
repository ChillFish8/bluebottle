//! Onboard a new Jellyfin media library.

use std::time::Duration;

use bluebottle_ui::{button, color, font, input, separator, spinner, title};
use iced::widget::text::IntoFragment;
use iced::widget::{Text, column, container, row, space, text};
use iced::{Center, Element, Length, padding, task};

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
    test_fail_reason: Option<String>,
    customisation_confirmed: bool,
    inflight_task: Option<task::Handle>,
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
    RetryTest,
    TestComplete(Result<(), String>),
}

impl view::View<JellyfinOnboardMsg> for JellyfinOnboard {
    fn update(&mut self, message: JellyfinOnboardMsg) -> task::Task<JellyfinOnboardMsg> {
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
                if !self.test_completed_successfully() {
                    return self.start_test();
                }
            },
            JellyfinOnboardMsg::NavigateCustomise => {
                self.navigate(Stage::Customise);
            },
            JellyfinOnboardMsg::ServerUrl(value) => {
                self.parsed_jellyfin_server_url = url::Url::parse(&value).ok();
                self.jellyfin_server_url = value;
                self.rest_test_state();
            },
            JellyfinOnboardMsg::Username(value) => {
                self.jellyfin_username = value;
                self.rest_test_state();
            },
            JellyfinOnboardMsg::Password(value) => {
                self.jellyfin_password = value;
                self.rest_test_state();
            },
            JellyfinOnboardMsg::TestComplete(result) => {
                self.test_failed = result.is_err();
                self.test_completed = true;
                self.test_fail_reason = result.err();
            },
            JellyfinOnboardMsg::RetryTest => {
                return self.start_test();
            },
        }

        task::Task::none()
    }

    fn view(&self) -> Element<'_, JellyfinOnboardMsg> {
        let subsection = match self.stage {
            Stage::AddServer => self.server_setup(),
            Stage::AddUser => self.user_setup(),
            Stage::Test => self.test_view(),
            Stage::Customise => space().into(),
            Stage::Complete => space().into(),
        };

        let wrapped_subsection = container(subsection).padding(padding::horizontal(8));

        column![self.navbar(), wrapped_subsection]
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
        self.inflight_task = None; // Cancel any inflight task.
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
        !self.jellyfin_username.is_empty() // Password is *technically* allowed to be empty.
    }

    /// Returns if the test is complete and it was successful.
    fn test_completed_successfully(&self) -> bool {
        self.test_completed && !self.test_failed
    }

    /// Returns whether the onboarding has been completed.
    fn is_complete(&self) -> bool {
        self.is_url_valid()
            && self.is_user_valid()
            && self.test_completed_successfully()
            && self.customisation_confirmed
    }

    /// Returns the parsed Jellyfin server URL.
    ///
    /// Panics if the URL is invalid.
    fn parsed_url(&self) -> &url::Url {
        self.parsed_jellyfin_server_url.as_ref().unwrap()
    }

    /// The reason why the latest test failed.
    fn test_fail_reason(&self) -> &str {
        self.test_fail_reason.as_deref().unwrap_or("unknown error")
    }

    fn rest_test_state(&mut self) {
        self.test_completed = false;
        self.test_failed = false;
        self.test_fail_reason = None;
    }

    fn start_test(&mut self) -> task::Task<JellyfinOnboardMsg> {
        self.rest_test_state();

        let fut = test_jellyfin_configuration(
            self.parsed_url().clone(),
            self.jellyfin_username.clone(),
            self.jellyfin_password.clone(),
        );

        let (task, handle) = task::Task::future(fut).abortable();
        self.inflight_task = Some(handle.abort_on_drop());

        task.map(JellyfinOnboardMsg::TestComplete)
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

    fn test_view(&self) -> Element<'_, JellyfinOnboardMsg> {
        if !self.test_completed {
            test_in_progress(self.parsed_url().as_str())
        } else if self.test_failed {
            test_failed(self.test_fail_reason())
        } else {
            test_success()
        }
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

fn info_message<'a>(info: impl IntoFragment<'a>) -> Text<'a> {
    text(info).color(color::TEXT_SECONDARY)
}

fn test_in_progress(address: &str) -> Element<'_, JellyfinOnboardMsg> {
    column![
        info_message(format!("Logging in to {address}")),
        spinner::linear(),
    ]
    .spacing(8)
    .into()
}

fn test_failed(reason: &str) -> Element<'_, JellyfinOnboardMsg> {
    let description = column![
        info_message("Bluebottle couldn't authenticate with the server."),
        info_message(format!("Reason: {reason}")),
        info_message("Please double check the server address and user info."),
        button::standard(
            "Retry",
            Some("refresh"),
            false,
            JellyfinOnboardMsg::RetryTest
        )
        .style(button::secondary_style)
    ]
    .spacing(8)
    .padding(padding::horizontal(2));

    column![
        title::title(Some("error"), "Something went wrong..."),
        description,
    ]
    .spacing(8)
    .into()
}

fn test_success() -> Element<'static, JellyfinOnboardMsg> {
    column![
        title::title(Some("check_circle"), "Success!"),
        container(info_message(
            "You're logged in, now we can setup your library."
        ))
        .padding(padding::horizontal(2)),
    ]
    .spacing(8)
    .into()
}

async fn test_jellyfin_configuration(
    _server: url::Url,
    _username: String,
    _password: String,
) -> Result<(), String> {
    tokio::time::sleep(Duration::from_secs(2)).await;
    Ok(())
}
