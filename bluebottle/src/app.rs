use bluebottle_ui::{bar, color, font};
use iced::widget::{column, container, row, space};
use iced::{Element, Length, Settings};

/// Run the Bluebottle UI iced application.
///
/// This will block until the user closes the application or the system crashes.
pub fn run_app() -> anyhow::Result<()> {
    let settings = Settings {
        fonts: font::required_fonts(),
        default_font: font::regular(),
        ..Default::default()
    };

    iced::application(|| Bluebottle::new(), Bluebottle::update, Bluebottle::view)
        .title("Bluebottle")
        .theme(color::theme())
        .settings(settings)
        .run()?;

    Ok(())
}

struct Bluebottle {
    screen: UIScreen,
}

enum GlobalMessage {}

impl Bluebottle {
    fn new() -> Self {
        Self {
            screen: UIScreen::Setup,
        }
    }

    fn update(&mut self, _message: GlobalMessage) {}

    fn view(&self) -> Element<'_, GlobalMessage> {
        match self.screen {
            UIScreen::Setup => self.setup_ui(),
            UIScreen::Library => self.library_ui(),
        }
    }

    fn library_ui(&self) -> Element<'_, GlobalMessage> {
        column![
            bar::top(space(), "Example Library"),
            row![
                bar::side(space(), space()),
                container(space()).width(Length::Fill).height(Length::Fill)
            ]
        ]
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
    }

    fn setup_ui(&self) -> Element<'_, GlobalMessage> {
        column![bar::top(space(), "Example Library"), row![]].into()
    }
}

#[derive(Copy, Clone, Debug)]
/// What UI screen the app should be displaying.
enum UIScreen {
    /// The user has no libraries available, we should onboard
    /// them with the setup screen.
    Setup,
    /// The user has some active libraries available, we should
    /// take them to that.
    Library,
}
