use bluebottle_ui::{bar, button, color, font};
use iced::advanced::graphics::text::cosmic_text::Align;
use iced::widget::{column, row, space};
use iced::{Center, Element, Settings, task};
use snafu::ResultExt;

use crate::navigator;
use crate::navigator::ActiveScreen;
use crate::screen::{Screen, library_select, library_view, loading, settings, setup};
use crate::view::View;

/// Run the Bluebottle UI iced application.
///
/// This will block until the user closes the application or the system crashes.
pub fn run_app() -> Result<(), snafu::Whatever> {
    navigator::load_from_state();
    //navigator::navigate(ActiveScreen::Loading);

    let settings = Settings {
        fonts: font::required_fonts(),
        default_font: font::regular(),
        ..Default::default()
    };

    iced::application(|| Bluebottle::new(), Bluebottle::update, Bluebottle::view)
        .title("Bluebottle")
        .theme(color::theme())
        .settings(settings)
        .run()
        .whatever_context("run Bluebottle main app")?;

    Ok(())
}

struct Bluebottle {
    library_view_screen: library_view::LibraryViewScreen,
    library_select_screen: library_select::LibrarySelectScreen,
    setup_screen: setup::SetupScreen,
    settings_screen: settings::SettingsScreen,
    loading_screen: loading::LoadingScreen,
}

#[derive(Clone)]
enum GlobalMessage {
    LibraryView(library_view::LibraryViewMsg),
    Loading(loading::LoadingMsg),
    Setup(setup::SetupMsg),
    LibrarySelect(library_select::LibrarySelectMsg),
    Settings(settings::SettingsMsg),
    Null,
}

impl Bluebottle {
    fn new() -> Self {
        Self {
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
            GlobalMessage::Null => task::Task::none(),
        }
    }

    fn view(&self) -> Element<'_, GlobalMessage> {
        column![
            self.render_topbar(),
            row![self.render_sidebar(), self.render_screen()],
        ]
        .into()
    }

    fn render_topbar(&self) -> Element<'_, GlobalMessage> {
        match navigator::active() {
            ActiveScreen::LibraryView => bar::top(
                self.library_view_screen
                    .nav_center()
                    .map(GlobalMessage::LibraryView),
                self.library_view_screen.nav_descriptor(),
            ),
            ActiveScreen::Loading => bar::top(
                self.loading_screen.nav_center().map(GlobalMessage::Loading),
                self.loading_screen.nav_descriptor(),
            ),
            ActiveScreen::Setup => bar::top(
                self.setup_screen.nav_center().map(GlobalMessage::Setup),
                self.setup_screen.nav_descriptor(),
            ),
            ActiveScreen::LibrarySelect => bar::top(
                self.library_select_screen
                    .nav_center()
                    .map(GlobalMessage::LibrarySelect),
                self.library_select_screen.nav_descriptor(),
            ),
            ActiveScreen::Settings => bar::top(
                self.settings_screen
                    .nav_center()
                    .map(GlobalMessage::Settings),
                self.settings_screen.nav_descriptor(),
            ),
        }
    }

    fn render_sidebar(&self) -> Element<'_, GlobalMessage> {
        if self.should_hide_sidebar() {
            return space().into();
        }

        let upper = column![
            button::nav("Home", "home", false, GlobalMessage::Null,),
            button::nav("Search", "search", false, GlobalMessage::Null,),
            button::nav("Favorites", "favorite", false, GlobalMessage::Null,),
            button::nav("Anime", "draw", false, GlobalMessage::Null),
            button::nav("TV Shows", "tv", false, GlobalMessage::Null),
            button::nav("Movies", "movie", false, GlobalMessage::Null),
            button::nav("Anime Movies", "movie", false, GlobalMessage::Null,),
            button::nav("Music", "library_music", false, GlobalMessage::Null,),
        ]
        .spacing(4)
        .align_x(Center);

        let lower = column![
            button::nav("Library", "apps", false, GlobalMessage::Null,),
            button::nav("Settings", "settings", true, GlobalMessage::Null,),
        ]
        .spacing(4)
        .align_x(Center);

        bar::side(upper, lower)
    }

    fn render_screen(&self) -> Element<'_, GlobalMessage> {
        match navigator::active() {
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

    fn should_hide_sidebar(&self) -> bool {
        match navigator::active() {
            ActiveScreen::LibraryView => library_view::LibraryViewScreen::HIDE_SIDEBAR,
            ActiveScreen::Loading => loading::LoadingScreen::HIDE_SIDEBAR,
            ActiveScreen::Setup => setup::SetupScreen::HIDE_SIDEBAR,
            ActiveScreen::LibrarySelect => {
                library_select::LibrarySelectScreen::HIDE_SIDEBAR
            },
            ActiveScreen::Settings => settings::SettingsScreen::HIDE_SIDEBAR,
        }
    }
}
