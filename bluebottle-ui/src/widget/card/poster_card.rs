//! 2:3 poster card. Shares the full watch-signal chrome with `EpisodeStill`
//! but for portrait artwork.

use std::borrow::Cow;

use iced::{Element, Size};

use super::frame::{self, Aspect, CardFrame};
use crate::widget::blur::Backdrop;
use crate::widget::button;

pub const DEFAULT_IMAGE_SIZE: Size = Size::new(178.0, 267.0);
pub const DEFAULT_RADIUS: f32 = 12.0;
/// Diameter of the centre play / replay button. See [`episode_still`].
pub const DEFAULT_PLAY_SIZE: f32 = button::AccentSizeVariant::Main.diameter();

pub fn poster_card<'a, Message>(backdrop: Backdrop) -> PosterCard<'a, Message>
where
    Message: Clone + 'a,
{
    PosterCard {
        backdrop,
        label: None,
        subtext: None,
        overlay: None,
        watched: false,
        favourite: false,
        progress: None,
        time_left: None,
        on_press: None,
        on_play: None,
        on_watched_toggled: None,
        on_favourite_toggled: None,
    }
}

pub struct PosterCard<'a, Message> {
    backdrop: Backdrop,
    label: Option<Element<'a, Message>>,
    subtext: Option<Element<'a, Message>>,
    overlay: Option<Element<'a, Message>>,
    watched: bool,
    favourite: bool,
    progress: Option<f32>,
    time_left: Option<Cow<'static, str>>,
    on_press: Option<Message>,
    on_play: Option<Message>,
    on_watched_toggled: Option<Box<dyn Fn(bool) -> Message + 'a>>,
    on_favourite_toggled: Option<Box<dyn Fn(bool) -> Message + 'a>>,
}

impl<'a, Message> PosterCard<'a, Message>
where
    Message: Clone + 'a,
{
    pub fn label(mut self, label: impl Into<Element<'a, Message>>) -> Self {
        self.label = Some(label.into());
        self
    }

    pub fn subtext(mut self, subtext: impl Into<Element<'a, Message>>) -> Self {
        self.subtext = Some(subtext.into());
        self
    }

    pub fn overlay(mut self, overlay: impl Into<Element<'a, Message>>) -> Self {
        self.overlay = Some(overlay.into());
        self
    }

    pub fn progress(
        mut self,
        progress: f32,
        time_left: impl Into<Cow<'static, str>>,
    ) -> Self {
        self.progress = Some(progress);
        self.time_left = Some(time_left.into());
        self
    }

    pub fn watched(mut self, watched: bool) -> Self {
        self.watched = watched;
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

    pub fn on_watched_toggled(mut self, emit: impl Fn(bool) -> Message + 'a) -> Self {
        self.on_watched_toggled = Some(Box::new(emit));
        self
    }

    pub fn on_favourite_toggled(mut self, emit: impl Fn(bool) -> Message + 'a) -> Self {
        self.on_favourite_toggled = Some(Box::new(emit));
        self
    }
}

impl<'a, Message> From<PosterCard<'a, Message>> for Element<'a, Message>
where
    Message: Clone + 'a,
{
    fn from(card: PosterCard<'a, Message>) -> Self {
        frame::build(CardFrame {
            backdrop: card.backdrop,
            aspect: Aspect::Portrait,
            image_size: DEFAULT_IMAGE_SIZE,
            corner_radius: DEFAULT_RADIUS,
            play_size: DEFAULT_PLAY_SIZE,
            watched: card.watched,
            favourite: card.favourite,
            progress: card.progress,
            time_left: card.time_left,
            label: card.label,
            subtext: card.subtext,
            overlay: card.overlay,
            on_press: card.on_press,
            on_play: card.on_play,
            on_watched_toggled: card.on_watched_toggled,
            on_favourite_toggled: card.on_favourite_toggled,
        })
    }
}
