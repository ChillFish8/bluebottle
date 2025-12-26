use bluebottle_ui::{button, input, pill, text};
use iced::widget::{column, row};
use iced::{Center, Element, Length, padding, task, widget};

use crate::view;

pub struct SettingsScreen {
    tab: Tab,
    scale_factor: f32,
}

#[derive(Clone)]
pub enum SettingsMsg {
    ChangeTab(Tab),
    UpdateScaleFactor(f32),
    Input(String),
}

#[derive(Default, Copy, Clone)]
enum Tab {
    #[default]
    General,
    Caching,
    Advanced,
}

impl Default for SettingsScreen {
    fn default() -> Self {
        Self {
            tab: Tab::default(),
            scale_factor: 1.0,
        }
    }
}

impl SettingsScreen {
    /// Returns the scale factor of the UI.
    pub fn scale_factor(&self) -> f32 {
        self.scale_factor
    }
}

impl super::Screen<SettingsMsg> for SettingsScreen {
    fn nav_descriptor(&self) -> &str {
        "Settings"
    }

    fn nav_center<'a>(&self) -> Element<'a, SettingsMsg>
    where
        SettingsMsg: 'a,
    {
        row![
            button::standard(
                "General",
                None,
                matches!(self.tab, Tab::General),
                SettingsMsg::ChangeTab(Tab::General)
            )
            .style(button::text_secondary_style),
            button::standard(
                "Caching",
                None,
                matches!(self.tab, Tab::Caching),
                SettingsMsg::ChangeTab(Tab::Caching)
            )
            .style(button::text_secondary_style),
            button::standard(
                "Advanced",
                None,
                matches!(self.tab, Tab::Advanced),
                SettingsMsg::ChangeTab(Tab::Advanced)
            )
            .style(button::text_secondary_style),
        ]
        .spacing(4)
        .height(Length::Fill)
        .into()
    }
}

impl view::View<SettingsMsg> for SettingsScreen {
    fn update(&mut self, message: SettingsMsg) -> task::Task<SettingsMsg> {
        match message {
            SettingsMsg::ChangeTab(tab) => self.tab = tab,
            SettingsMsg::UpdateScaleFactor(factor) => {
                self.scale_factor = (factor / 100.0)
            },
            SettingsMsg::Input(_input) => {},
        }
        task::Task::none()
    }

    fn view(&self) -> Element<'_, SettingsMsg> {
        column![
            column![
                text::title(None, "UI Scaling"),
                text::paragraph(
                    "Alters the relative size/zoom of all UI components."
                ),
                row![
                    text::label("50%"),
                    widget::slider(
                        50.0..=200.0,
                        100.0 * self.scale_factor,
                        SettingsMsg::UpdateScaleFactor,
                    ),
                    text::label("200%"),
                    pill::small(format!("Current: {:.0}%", self.scale_factor * 100.0), None)
                ].width(500).spacing(4).align_y(Center),
                button::standard("Reset", Some("replay"), false, SettingsMsg::UpdateScaleFactor(100.0))
                .style(button::secondary_style)
            ].spacing(4),
            column![
                text::title(None, "Playback Settings"),
                text::paragraph(
                    "Adjust default audio and video streams, subtitle selection, and other \
                    settings related to the Bluebottle playback engine."
                ),
            ].spacing(4),
            column![
                text::title(None, "Advanced Settings"),
                text::paragraph(
                    "Adjust advanced options of the video engine including hardware acceleration, \
                    timeouts and HDR options."
                ),
            ].spacing(4),
        ]
            .spacing(16)
            .padding(8)
            .width(Length::Fill)
            .into()
    }
}
