use bluebottle_ui::{bar, color, font};
use iced::widget::{column, row, space};
use iced::{Element, Settings, task};

use crate::screen::{library_select, library_view, loading, settings, setup};
use crate::view::View;

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
    screen: ActiveScreen,
    library_view_screen: library_view::LibraryViewScreen,
    library_select_screen: library_select::LibrarySelectScreen,
    setup_screen: setup::SetupScreen,
    settings_screen: settings::SettingsScreen,
    loading_screen: loading::LoadingScreen,
}

enum GlobalMessage {
    LibraryView(library_view::LibraryViewMsg),
    Loading(loading::LoadingMsg),
    Setup(setup::SetupMsg),
    LibrarySelect(library_select::LibrarySelectMsg),
    Settings(settings::SettingsMsg),
}

impl Bluebottle {
    fn new() -> Self {
        Self {
            screen: ActiveScreen::Setup,
            library_view_screen: library_view::LibraryViewScreen::default(),
            library_select_screen: library_select::LibrarySelectScreen::default(),
            setup_screen: setup::SetupScreen::default(),
            settings_screen: settings::SettingsScreen::default(),
            loading_screen: loading::LoadingScreen::default(),
        }
    }

    fn update(&mut self, message: GlobalMessage) -> task::Task<GlobalMessage> {
        match message {
            GlobalMessage::LibraryView(msg) => self
                .library_view_screen
                .update(msg)
                .map(GlobalMessage::LibraryView),
            GlobalMessage::Loading(msg) => {
                self.loading_screen.update(msg).map(GlobalMessage::Loading)
            },
            GlobalMessage::Setup(msg) => {
                self.setup_screen.update(msg).map(GlobalMessage::Setup)
            },
            GlobalMessage::LibrarySelect(msg) => self
                .library_select_screen
                .update(msg)
                .map(GlobalMessage::LibrarySelect),
            GlobalMessage::Settings(msg) => self
                .settings_screen
                .update(msg)
                .map(GlobalMessage::Settings),
        }
    }

    fn view(&self) -> Element<'_, GlobalMessage> {
        match self.screen {
            ActiveScreen::LibraryView => self
                .library_view_screen
                .view()
                .map(GlobalMessage::LibraryView),
            ActiveScreen::Loading => {
                self.loading_screen.view().map(GlobalMessage::Loading)
            },
            ActiveScreen::Setup => self.setup_screen.view().map(GlobalMessage::Setup),
            ActiveScreen::LibrarySelect => self
                .library_select_screen
                .view()
                .map(GlobalMessage::LibrarySelect),
            ActiveScreen::Settings => {
                self.settings_screen.view().map(GlobalMessage::Settings)
            },
        }
    }

    fn setup_ui(&self) -> Element<'_, GlobalMessage> {
        column![bar::top(space(), "Example Library"), row![]].into()
    }
}

#[derive(Copy, Clone, Debug)]
/// What UI screen the app should be displaying.
enum ActiveScreen {
    /// View an existing media library.
    LibraryView,
    /// The library being requested is still being prepared, show
    /// the user a loading screen for now.
    Loading,
    /// The user has no libraries available, we should onboard
    /// them with the setup screen.
    Setup,
    /// Select an existing media library (or add a new one.)
    LibrarySelect,
    /// View the app settings.
    Settings,
}
