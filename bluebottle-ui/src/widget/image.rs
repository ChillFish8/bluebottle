use iced::ContentFit;
use iced::widget::image;
pub use image::{Handle, Image};

/// The sizing options of the poster.
pub enum PosterSize {
    Small,
    Medium,
    Large,
}

/// An image in the poster aspect ratio.
pub fn poster(handle: Handle, size: PosterSize) -> Image {
    let (width, height) = match size {
        PosterSize::Small => (152, 224),
        PosterSize::Medium => (175, 256),
        PosterSize::Large => (240, 352),
    };

    image(handle)
        .width(width)
        .height(height)
        .content_fit(ContentFit::Cover)
}

/// An image in the thumbnail aspect ratio.
pub fn thumbnail(handle: Handle) -> Image {
    image(handle)
        .width(270)
        .height(152)
        .content_fit(ContentFit::Cover)
        .into()
}

/// An image in the square aspect ratio.
pub fn square(handle: Handle) -> Image {
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
pub fn person(handle: Handle, size: PersonSize) -> Image {
    match size {
        PersonSize::Square => square(handle),
        PersonSize::Poster => poster(handle, PosterSize::Small),
    }
}
