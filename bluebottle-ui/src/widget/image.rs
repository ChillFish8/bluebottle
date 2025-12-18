use iced::widget::image;
use iced::{ContentFit, Element};
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

/// Creates a loading skeleton for the poster form factor.
pub fn poster_skeleton<'a, Message>(size: PosterSize) -> Element<'a, Message>
where
    Message: Clone + 'a,
{
    let (width, height) = match size {
        PosterSize::Small => (152, 224),
        PosterSize::Medium => (175, 256),
        PosterSize::Large => (240, 352),
    };

    super::skeleton::skeleton()
        .width(width)
        .height(height)
        .into()
}

/// An image in the thumbnail aspect ratio.
pub fn thumbnail(handle: Handle) -> Image {
    image(handle)
        .width(270)
        .height(152)
        .content_fit(ContentFit::Cover)
        .into()
}

/// Creates a loading skeleton for the thumbnail form factor.
pub fn thumbnail_skeleton<'a, Message>() -> Element<'a, Message>
where
    Message: Clone + 'a,
{
    super::skeleton::skeleton().width(270).height(152).into()
}

/// An image in the square aspect ratio.
pub fn square(handle: Handle) -> Image {
    image(handle)
        .width(152)
        .height(152)
        .content_fit(ContentFit::Cover)
        .into()
}

/// Creates a loading skeleton for the square form factor.
pub fn square_skeleton<'a, Message>() -> Element<'a, Message>
where
    Message: Clone + 'a,
{
    super::skeleton::skeleton().width(152).height(152).into()
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

/// Creates a loading skeleton for the person form factor.
pub fn person_skeleton<'a, Message>(size: PersonSize) -> Element<'a, Message>
where
    Message: Clone + 'a,
{
    match size {
        PersonSize::Square => square_skeleton(),
        PersonSize::Poster => poster_skeleton(PosterSize::Small),
    }
}
