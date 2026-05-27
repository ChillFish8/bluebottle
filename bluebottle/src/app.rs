use std::sync::Arc;

use bluebottle_video::Player;
use iced::keyboard::key::Named;
use iced::keyboard::{Event as KeyboardEvent, Key};
use iced::{Element, Event, Subscription, Task};

use crate::background::BackgroundSource;
use crate::screen::main::{MainMsg, MainScreen};
use crate::screen::player::{PlayerMsg, PlayerScreen};

/// Which view is shown.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Screen {
    /// The main library surface: opaque, image-derived background.
    Main,
    /// Video playback: a transparent overlay over the libplacebo surface.
    Player,
}

/// The application state, owning each screen.
pub struct App {
    screen: Screen,
    main: MainScreen,
    player: PlayerScreen,
}

/// Messages produced by the UI and subscriptions, wrapping per-screen messages.
#[derive(Debug, Clone)]
pub enum Message {
    Main(MainMsg),
    Player(PlayerMsg),
    /// Toggle between the main and player screens.
    ToggleScreen,
}

impl App {
    /// Builds the app on the [`Main`](Screen::Main) screen with `source` as its
    /// background, ready to switch to `player` on demand.
    pub fn new(player: Arc<Player>, source: Arc<BackgroundSource>) -> Self {
        Self {
            screen: Screen::Main,
            main: MainScreen::new(source),
            player: PlayerScreen::new(player),
        }
    }

    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::Main(message) => self.main.update(message).map(Message::Main),
            Message::Player(message) => self.player.update(message).map(Message::Player),
            Message::ToggleScreen => {
                self.screen = match self.screen {
                    Screen::Main => {
                        self.player.play();
                        Screen::Player
                    },
                    Screen::Player => {
                        self.player.stop();
                        Screen::Main
                    },
                };
                Task::none()
            },
        }
    }

    pub fn view(&self) -> Element<'_, Message> {
        match self.screen {
            Screen::Main => self.main.view().map(Message::Main),
            Screen::Player => self.player.view().map(Message::Player),
        }
    }

    pub fn subscription(&self) -> Subscription<Message> {
        Subscription::batch([
            iced::event::listen_with(toggle_on_space),
            self.main.subscription().map(Message::Main),
        ])
    }
}

/// Maps a <kbd>Space</kbd> press to [`Message::ToggleScreen`], ignoring the rest.
fn toggle_on_space(
    event: Event,
    _status: iced::event::Status,
    _window: iced::window::Id,
) -> Option<Message> {
    match event {
        Event::Keyboard(KeyboardEvent::KeyPressed {
            key: Key::Named(Named::Space),
            ..
        }) => Some(Message::ToggleScreen),
        _ => None,
    }
}
