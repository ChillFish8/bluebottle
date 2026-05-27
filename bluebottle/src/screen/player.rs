use std::sync::Arc;

use bluebottle_video::Player;
use iced::widget::Space;
use iced::{Element, Length, Task};

/// Messages emitted by the player overlay.
#[derive(Debug, Clone)]
pub enum PlayerMsg {}

/// State for the player overlay.
pub struct PlayerScreen {
    player: Arc<Player>,
}

impl PlayerScreen {
    pub fn new(player: Arc<Player>) -> Self {
        Self { player }
    }

    /// Starts playback when the player screen becomes active.
    pub fn play(&self) {
        if let Err(error) = self.player.play() {
            tracing::error!(%error, "failed to start playback");
        }
    }

    /// Stops the pipeline when leaving the player screen.
    pub fn stop(&self) {
        self.player.stop();
    }

    pub fn update(&mut self, message: PlayerMsg) -> Task<PlayerMsg> {
        match message {}
    }

    pub fn view(&self) -> Element<'_, PlayerMsg> {
        Space::new().width(Length::Fill).height(Length::Fill).into()
    }
}
