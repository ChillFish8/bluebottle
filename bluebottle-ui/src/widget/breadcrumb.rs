use std::marker::PhantomData;

use iced::widget::Row;
use iced::{Center, Element};

use crate::{color, text};

/// A breadcrumb trail. The last entry reads as the current section in the
/// default text colour. Earlier entries and the separators read muted.
pub fn breadcrumb<'a, Message>(crumbs: &'a [&'a str]) -> Breadcrumb<'a, Message>
where
    Message: 'a,
{
    Breadcrumb {
        crumbs,
        _message: PhantomData,
    }
}

/// A configurable breadcrumb, built by [`breadcrumb`].
pub struct Breadcrumb<'a, Message> {
    crumbs: &'a [&'a str],
    _message: PhantomData<Message>,
}

impl<'a, Message> From<Breadcrumb<'a, Message>> for Element<'a, Message>
where
    Message: 'a,
{
    fn from(breadcrumb: Breadcrumb<'a, Message>) -> Self {
        let last = breadcrumb.crumbs.len().saturating_sub(1);

        let items =
            breadcrumb
                .crumbs
                .iter()
                .enumerate()
                .flat_map(move |(index, crumb)| {
                    let color = if index == last {
                        color::TEXT_PRIMARY
                    } else {
                        color::TEXT_MUTED
                    };

                    let crumb = text::caption(*crumb).color(color).into();

                    if index == 0 {
                        vec![crumb]
                    } else {
                        let sep = text::caption("/").color(color::TEXT_MUTED).into();

                        vec![sep, crumb]
                    }
                });

        Row::with_children(items)
            .spacing(5.5)
            .align_y(Center)
            .into()
    }
}
