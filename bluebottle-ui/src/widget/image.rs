use iced::widget::image;
use iced::{ContentFit, Element};

/// The sizing options of the poster.
pub enum PosterSize {
    Small,
    Medium,
    Large,
}

/// An image in the poster aspect ratio.
pub fn poster<'a, Message>(
    handle: image::Handle,
    size: PosterSize,
) -> Element<'a, Message> {
    let (width, height) = match size {
        PosterSize::Small => (152, 224),
        PosterSize::Medium => (175, 256),
        PosterSize::Large => (240, 352),
    };

    image(handle)
        .width(width)
        .height(height)
        .content_fit(ContentFit::Cover)
        .into()
}

/// An image in the episode aspect ratio.
pub fn episode<'a, Message>(handle: image::Handle) -> Element<'a, Message> {
    image(handle)
        .width(276)
        .height(145.67)
        .content_fit(ContentFit::Cover)
        .into()
}

/// An image in the album aspect ratio.
pub fn album<'a, Message>(handle: image::Handle) -> Element<'a, Message> {
    image(handle)
        .width(152)
        .height(152)
        .content_fit(ContentFit::Cover)
        .into()
}

/// The sizing options of the person image.
pub enum PersonSize {
    Square,
    Poster,
}

/// An image in the person aspect ratio.
pub fn person<'a, Message>(
    handle: image::Handle,
    size: PersonSize,
) -> Element<'a, Message> {
    let (width, height) = match size {
        PersonSize::Square => (152, 152),
        PersonSize::Poster => (152, 224),
    };

    image(handle)
        .width(width)
        .height(height)
        .content_fit(ContentFit::Cover)
        .into()
}
