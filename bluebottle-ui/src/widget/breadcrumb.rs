use iced::widget::{Row, row, text};
use iced::{Center, Element};

use crate::color;

/// A breadcrumb trail. The last entry reads as the current section in the
/// default text colour. Earlier entries and the separators read as secondary.
pub fn breadcrumb<'a, Message>(crumbs: &'a [&'a str]) -> Row<'a, Message>
where
    Message: 'a,
{
    let last = crumbs.len().saturating_sub(1);

    let items = crumbs.iter().enumerate().flat_map(|(index, crumb)| {
        let color = if index == last {
            color::TEXT_DEFAULT
        } else {
            color::TEXT_SECONDARY
        };

        let crumb: Element<'a, Message> = text(*crumb).color(color).into();

        if index == 0 {
            vec![crumb]
        } else {
            let sep: Element<'a, Message> =
                text("/").color(color::TEXT_SECONDARY).into();
            vec![sep, crumb]
        }
    });

    Row::with_children(items).spacing(8).align_y(Center)
}
