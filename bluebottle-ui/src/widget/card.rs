use std::time::Duration;

use iced::widget::{self, column, container, hover, stack, text};
use iced::{Background, Border, Center, Element, Length, Theme};

use super::image::PosterSize;
use super::{button, image};
use crate::color::{TEXT_DEFAULT, TEXT_SECONDARY};
use crate::{color, icon, util};

#[derive(Copy, Clone)]
/// The form factor of the card.
pub enum CardFormFactor {
    Poster,
    Thumbnail,
    Square,
}

#[derive(Copy, Clone)]
/// The messages kind that can be triggered by the playable card.
pub enum PlayableMessageKind {
    /// The user has clicked the play button.
    Play,
    /// The user has clicked the general card.
    Inspect,
    /// The user has toggled the "watched" marker.
    ToggleWatched,
}

#[derive(Copy, Clone, Eq, PartialEq)]
/// The current state of the last user interaction with the card.
pub enum WatchState {
    /// The user has watched this media.
    Watched,
    /// The user has started but not completed this media.
    InProgress,
    /// The user has yet to watch this media.
    Unwatched,
}

#[derive(Copy, Clone)]
/// A message that is produced by events on the playable card.
pub struct PlayableCardMessage {
    /// The unique ID assigned to the card.
    pub id: usize,
    /// The kind of message that occurred.
    pub kind: PlayableMessageKind,
}

impl PlayableCardMessage {
    fn new(id: usize, kind: PlayableMessageKind) -> Self {
        Self { id, kind }
    }
}

/// Information to render a playable card.
pub struct PlayableCardInfo<'a> {
    /// The title that appears just under the display image.
    pub label: &'a str,
    /// The subtext that appears just under the title.
    pub subtext: &'a str,
    /// The display image being rendered.
    pub image: image::Handle,
    /// What state was the user interaction in.
    pub watch_state: WatchState,
    /// The runtime of the media if applicable.
    pub runtime: Option<Duration>,
}

/// A piece of media that can be played.
pub fn playable<'a, Message>(
    id: usize,
    info: PlayableCardInfo<'a>,
    form: CardFormFactor,
) -> Element<'a, Message>
where
    Message: From<PlayableCardMessage> + Clone + 'a,
{
    if info.watch_state == WatchState::Watched {
        return watched(id, info, form);
    }

    let [play_msg, inspect_msg, watched_msg] = create_playable_messages(id);

    let display_image = display_image(info.image, form);
    let image_size = util::widget_size::<Message>(&display_image);

    let play_icon = icon::filled("play_circle").size(32);
    let play_button = button::icon(play_icon, false, play_msg);
    let play_button_container = container(play_button)
        .width(Length::Fill)
        .height(image_size.height)
        .align_x(Center)
        .align_y(Center)
        .style(shader_style);

    let label = text(info.label).size(14).color(TEXT_DEFAULT);
    let subtext = text(info.subtext).size(12).color(TEXT_SECONDARY);

    let base = column![display_image.border_radius(8), label, subtext].align_x(Center);

    // note: the padding is needed due to a clipping issue in the layout engine of iced (I think)
    widget::button(container(hover(base, play_button_container)).padding(1))
        .on_press(inspect_msg)
        .style(wrapping_button_style)
        .into()
}

fn watched<'a, Message>(
    id: usize,
    info: PlayableCardInfo<'a>,
    form: CardFormFactor,
) -> Element<'a, Message>
where
    Message: From<PlayableCardMessage> + Clone + 'a,
{
    let [play_msg, inspect_msg, watched_msg] = create_playable_messages(id);

    let display_image = display_image(info.image, form);
    let image_size = util::widget_size::<Message>(&display_image);

    let play_icon = icon::filled("replay").size(32);
    let play_button = button::icon(play_icon, false, play_msg);
    let play_button_container = container(play_button)
        .width(Length::Fill)
        .height(image_size.height)
        .align_x(Center)
        .align_y(Center)
        .style(|theme| {
            let mut style = shader_style(theme);
            style.border = Border::default().rounded(8);
            style
        });

    let display_container =
        stack![display_image.border_radius(8), play_button_container,];

    let border_overlay = container("")
        .width(Length::Fill)
        .height(image_size.height)
        .style(|theme| {
            let mut style = shader_style(theme);
            style.background = None;
            style
        });

    let label = text(info.label).size(14).color(TEXT_DEFAULT);
    let subtext = text(info.subtext).size(12).color(TEXT_SECONDARY);

    let base = column![display_container, label, subtext].align_x(Center);

    // note: the padding is needed due to a clipping issue in the layout engine of iced (I think)
    widget::button(container(hover(base, border_overlay)).padding(1))
        .on_press(inspect_msg)
        .style(wrapping_button_style)
        .into()
}

/// A display image and label text that can be clicked.
pub fn clickable<'a, Message>(
    label: &'a str,
    subtext: &'a str,
    image: image::Handle,
    form: CardFormFactor,
    on_click: Message,
) -> Element<'a, Message>
where
    Message: Clone + 'a,
{
    let display_image = display_image(image, form);

    let overlay = container("")
        .width(Length::Fill)
        .height(util::widget_size::<Message>(&display_image).height)
        .align_x(Center)
        .align_y(Center)
        .style(shader_style);

    let label = text(label).size(14).color(TEXT_DEFAULT);
    let subtext = text(subtext).size(12).color(TEXT_SECONDARY);

    let base = column![display_image.border_radius(8), label, subtext].align_x(Center);

    // note: the padding is needed due to a clipping issue in the layout engine of iced (I think)
    widget::button(container(hover(base, overlay)).padding(1))
        .on_press(on_click)
        .style(wrapping_button_style)
        .into()
}

fn display_image(image: image::Handle, form_factor: CardFormFactor) -> image::Image {
    match form_factor {
        CardFormFactor::Poster => image::poster(image, PosterSize::Small),
        CardFormFactor::Thumbnail => image::thumbnail(image),
        CardFormFactor::Square => image::square(image),
    }
}

/// Creates a new set of messages for the given [PlayableMessageKind] variants.
fn create_playable_messages<'a, Message>(id: usize) -> [Message; 3]
where
    Message: From<PlayableCardMessage> + 'a,
{
    let play_message =
        Message::from(PlayableCardMessage::new(id, PlayableMessageKind::Play));
    let inspect_message =
        Message::from(PlayableCardMessage::new(id, PlayableMessageKind::Inspect));
    let watched_message = Message::from(PlayableCardMessage::new(
        id,
        PlayableMessageKind::ToggleWatched,
    ));

    [play_message, inspect_message, watched_message]
}

fn shader_style(_theme: &Theme) -> container::Style {
    container::Style {
        text_color: None,
        background: Some(Background::Color(color::BACKGROUND).scale_alpha(0.8)),
        border: Border::default().width(1).rounded(8).color(color::PRIMARY),
        shadow: Default::default(),
        snap: true,
    }
}

fn wrapping_button_style(_theme: &Theme, _status: button::Status) -> button::Style {
    button::Style {
        background: None,
        text_color: Default::default(),
        border: Default::default(),
        shadow: Default::default(),
        snap: true,
    }
}
