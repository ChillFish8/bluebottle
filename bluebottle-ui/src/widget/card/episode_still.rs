//! 16:9 episode still card with the full watch-signal chrome (progress strip,
//! time-left pill, watched checkbox pill, favourite heart, accent-glass play).

use std::borrow::Cow;

use iced::{Element, Size};

use super::frame::{self, Aspect, CardFrame};
use crate::widget::blur::Backdrop;
use crate::widget::button;

pub const DEFAULT_IMAGE_SIZE: Size = Size::new(320.0, 180.0);
pub const DEFAULT_RADIUS: f32 = 12.0;
/// Diameter of the centre play button. Sourced from the button widget
/// so the frosted backdrop stays concentric with the real button.
pub const DEFAULT_PLAY_SIZE: f32 = button::AccentSizeVariant::Main.diameter();

/// Creates an episode still over `backdrop`. Captions and chrome are opt-in
/// through the builder.
pub fn episode_still<'a, Message>(backdrop: Backdrop) -> EpisodeStill<'a, Message>
where
    Message: Clone + 'a,
{
    EpisodeStill {
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

pub struct EpisodeStill<'a, Message> {
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

impl<'a, Message> EpisodeStill<'a, Message>
where
    Message: Clone + 'a,
{
    /// Primary caption row below the image. Accepts any element so callers
    /// can pass a [`link`](crate::widget::link::link) for clickable titles.
    pub fn label(mut self, label: impl Into<Element<'a, Message>>) -> Self {
        self.label = Some(label.into());
        self
    }

    /// Secondary caption row below the label.
    pub fn subtext(mut self, subtext: impl Into<Element<'a, Message>>) -> Self {
        self.subtext = Some(subtext.into());
        self
    }

    /// Caller-supplied overlay drawn on top of the chrome.
    pub fn overlay(mut self, overlay: impl Into<Element<'a, Message>>) -> Self {
        self.overlay = Some(overlay.into());
        self
    }

    /// Mid-watch state. `progress` is in `[0, 1]`; `time_left` is the
    /// free-form label rendered alongside the clock glyph.
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

/// Shimmer placeholder with the episode still's image and caption layout.
/// Drop in while the backdrop is still loading so the page holds its shape.
pub fn episode_still_skeleton<'a, Message>() -> Element<'a, Message>
where
    Message: 'a,
{
    frame::skeleton(DEFAULT_IMAGE_SIZE, DEFAULT_RADIUS)
}

impl<'a, Message> From<EpisodeStill<'a, Message>> for Element<'a, Message>
where
    Message: Clone + 'a,
{
    fn from(card: EpisodeStill<'a, Message>) -> Self {
        frame::build(CardFrame {
            backdrop: card.backdrop,
            aspect: Aspect::Landscape,
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
