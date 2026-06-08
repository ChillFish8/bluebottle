//! 1:1 album card. The smallest of the three. Carries play and favourite
//! only; no progress, no watched, no time-left.

use iced::{Element, Size};

use super::frame::{self, Aspect, CardFrame};
use crate::border;
use crate::widget::blur::Backdrop;
use crate::widget::button;

pub const DEFAULT_IMAGE_SIZE: Size = Size::new(150.0, 150.0);
/// Diameter of the centre play / replay button. See [`episode_still`].
pub const DEFAULT_PLAY_SIZE: f32 = button::AccentSizeVariant::Main.diameter();

pub fn album_card<'a, Message>(backdrop: Backdrop) -> AlbumCard<'a, Message>
where
    Message: Clone + 'a,
{
    AlbumCard {
        backdrop,
        label: None,
        subtext: None,
        overlay: None,
        favourite: false,
        on_press: None,
        on_play: None,
        on_favourite_toggled: None,
    }
}

pub struct AlbumCard<'a, Message> {
    backdrop: Backdrop,
    label: Option<Element<'a, Message>>,
    subtext: Option<Element<'a, Message>>,
    overlay: Option<Element<'a, Message>>,
    favourite: bool,
    on_press: Option<Message>,
    on_play: Option<Message>,
    on_favourite_toggled: Option<Box<dyn Fn(bool) -> Message + 'a>>,
}

impl<'a, Message> AlbumCard<'a, Message>
where
    Message: Clone + 'a,
{
    /// Primary caption row below the image (typically the album title).
    pub fn label(mut self, label: impl Into<Element<'a, Message>>) -> Self {
        self.label = Some(label.into());
        self
    }

    /// Secondary caption row (typically the artist).
    pub fn subtext(mut self, subtext: impl Into<Element<'a, Message>>) -> Self {
        self.subtext = Some(subtext.into());
        self
    }

    /// Caller-supplied overlay drawn on top of the chrome.
    pub fn overlay(mut self, overlay: impl Into<Element<'a, Message>>) -> Self {
        self.overlay = Some(overlay.into());
        self
    }

    pub fn favourite(mut self, favourite: bool) -> Self {
        self.favourite = favourite;
        self
    }

    pub fn on_press(mut self, message: Message) -> Self {
        self.on_press = Some(message);
        self
    }

    pub fn on_play(mut self, message: Message) -> Self {
        self.on_play = Some(message);
        self
    }

    pub fn on_favourite_toggled(mut self, emit: impl Fn(bool) -> Message + 'a) -> Self {
        self.on_favourite_toggled = Some(Box::new(emit));
        self
    }
}

/// Shimmer placeholder with the album card's image and caption layout.
pub fn album_card_skeleton<'a, Message>() -> Element<'a, Message>
where
    Message: 'a,
{
    frame::skeleton(DEFAULT_IMAGE_SIZE, border::ROUNDED_LG)
}

impl<'a, Message> From<AlbumCard<'a, Message>> for Element<'a, Message>
where
    Message: Clone + 'a,
{
    fn from(card: AlbumCard<'a, Message>) -> Self {
        frame::build(CardFrame {
            backdrop: card.backdrop,
            aspect: Aspect::Square,
            image_size: DEFAULT_IMAGE_SIZE,
            corner_radius: border::ROUNDED_LG,
            play_size: DEFAULT_PLAY_SIZE,
            watched: false,
            favourite: card.favourite,
            progress: None,
            time_left: None,
            label: card.label,
            subtext: card.subtext,
            overlay: card.overlay,
            on_press: card.on_press,
            on_play: card.on_play,
            on_watched_toggled: None,
            on_favourite_toggled: card.on_favourite_toggled,
        })
    }
}
