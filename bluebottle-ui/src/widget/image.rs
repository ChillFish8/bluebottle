use iced::widget::image;
use iced::{ContentFit, Element};

pub use image::Handle;

/// The sizing options of the poster.
pub enum PosterSize {
    Small,
    Medium,
    Large,
}

/// An image in the poster aspect ratio.
pub fn poster<'a, Message>(
    handle: Handle,
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

/// An image in the thumbnail aspect ratio.
pub fn thumbnail<'a, Message>(handle: Handle) -> Element<'a, Message> {
    image(handle)
        .width(276)
        .height(145.67)
        .content_fit(ContentFit::Cover)
        .into()
}

/// An image in the square aspect ratio.
pub fn square<'a, Message>(handle: Handle) -> Element<'a, Message> {
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
    handle: Handle,
    size: PersonSize,
) -> Element<'a, Message> {
    match size {
        PersonSize::Square => square(handle),
        PersonSize::Poster => poster(handle, PosterSize::Small),
    }
}
