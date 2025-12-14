use iced::widget::{self, column, container, hover, text};
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

/// A piece of media that can be played.
pub fn playable<'a, Message>(
    label: &'a str,
    subtext: &'a str,
    image: image::Handle,
    form: CardFormFactor,
    on_inspect: Message,
    on_play: Message,
) -> Element<'a, Message>
where
    Message: Clone + 'a,
{
    let display_image = match form {
        CardFormFactor::Poster => image::poster(image, PosterSize::Small),
        CardFormFactor::Thumbnail => image::thumbnail(image),
        CardFormFactor::Square => image::square(image),
    };

    let play_icon = icon::filled("play_circle").size(32);
    let play_button = button::icon(play_icon, false, on_play);
    let play_button_container = container(play_button)
        .width(Length::Fill)
        .height(util::widget_size::<Message>(&display_image).height)
        .align_x(Center)
        .align_y(Center)
        .style(shader_style);

    let label = text(label).size(14).color(TEXT_DEFAULT);
    let subtext = text(subtext).size(12).color(TEXT_SECONDARY);

    let base = column![display_image.border_radius(8), label, subtext].align_x(Center);

    // note: the padding is needed due to a clipping issue in the layout engine of iced (I think)
    widget::button(container(hover(base, play_button_container)).padding(1))
        .on_press(on_inspect)
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
    let display_image = match form {
        CardFormFactor::Poster => image::poster(image, PosterSize::Small),
        CardFormFactor::Thumbnail => image::thumbnail(image),
        CardFormFactor::Square => image::square(image),
    };

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
