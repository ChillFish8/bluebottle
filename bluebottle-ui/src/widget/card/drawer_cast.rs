//! Compact cast row for the slide-in drawer. A 44px circular avatar leads
//! the name and role pair. Bare ground, no hover, no chevron, no press.
//! Use over [`continue_watching`](super::continue_watching) cards when the
//! narrower drawer needs a list-item rhythm instead of a tile rhythm.

use iced::widget::text::IntoFragment;
use iced::widget::{column, container, image, row};
use iced::{Center, ContentFit, Element, Length, padding};

use crate::widget::text;
use crate::{color, font};

const AVATAR_SIZE: f32 = 44.0;
const ROW_GAP: f32 = 12.0;
const TEXT_GAP: f32 = 2.0;

/// A bare cast row. `name` reads as the Card Title, `role` as the caption
/// beneath. The avatar is rendered without a ring so the drawer's quiet
/// chrome stays uniform.
pub fn drawer_cast_row<'a, Message>(
    avatar: image::Handle,
    name: impl IntoFragment<'a>,
    role: impl IntoFragment<'a>,
) -> Element<'a, Message>
where
    Message: 'a,
{
    let avatar = image(avatar)
        .width(AVATAR_SIZE)
        .height(AVATAR_SIZE)
        .content_fit(ContentFit::Cover)
        .border_radius(AVATAR_SIZE * 0.5);

    let identity = column![
        text::card_title(name)
            .font(font::semibold())
            .color(color::TEXT_PRIMARY),
        text::caption(role),
    ]
    .spacing(TEXT_GAP);

    container(
        row![avatar, identity]
            .spacing(ROW_GAP)
            .align_y(Center)
            .width(Length::Fill),
    )
    .width(Length::Fill)
    .padding(padding::Padding::default().vertical(10).horizontal(12))
    .into()
}

/// Shimmer placeholder matching the row layout. A circle for the avatar
/// beside two stacked bars for name and role. Drop in while the cast list
/// is still loading so the drawer holds its rhythm.
pub fn drawer_cast_row_skeleton<'a, Message>() -> Element<'a, Message>
where
    Message: 'a,
{
    use crate::widget::skeleton::skeleton as shimmer;

    let avatar: Element<'a, Message> = shimmer()
        .width(Length::Fixed(AVATAR_SIZE))
        .height(Length::Fixed(AVATAR_SIZE))
        .radius(AVATAR_SIZE * 0.5)
        .into();

    let name_bar: Element<'a, Message> = shimmer()
        .width(Length::Fixed(140.0))
        .height(Length::Fixed(13.0))
        .radius(4.0)
        .into();

    let role_bar: Element<'a, Message> = shimmer()
        .width(Length::Fixed(80.0))
        .height(Length::Fixed(11.0))
        .radius(4.0)
        .into();

    let identity = column![name_bar, role_bar].spacing(TEXT_GAP);

    container(
        row![avatar, identity]
            .spacing(ROW_GAP)
            .align_y(Center)
            .width(Length::Fill),
    )
    .width(Length::Fill)
    .padding(padding::Padding::default().vertical(10).horizontal(12))
    .into()
}
