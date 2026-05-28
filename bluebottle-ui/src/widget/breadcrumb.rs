use std::marker::PhantomData;

use iced::widget::{Row, text};
use iced::{Center, Element, Pixels};

use crate::{color, font};

/// A breadcrumb trail. The last entry reads as the current section in the
/// default text colour. Earlier entries and the separators read muted.
pub fn breadcrumb<'a, Message>(crumbs: &'a [&'a str]) -> Breadcrumb<'a, Message>
where
    Message: 'a,
{
    Breadcrumb {
        crumbs,
        size: font::TEXT_MEDIUM.into(),
        _message: PhantomData,
    }
}

/// A configurable breadcrumb, built by [`breadcrumb`].
pub struct Breadcrumb<'a, Message> {
    crumbs: &'a [&'a str],
    size: Pixels,
    _message: PhantomData<Message>,
}

impl<'a, Message> Breadcrumb<'a, Message> {
    /// Sets the text size for every crumb and separator.
    pub fn size(mut self, size: impl Into<Pixels>) -> Self {
        self.size = size.into();
        self
    }
}

impl<'a, Message> From<Breadcrumb<'a, Message>> for Element<'a, Message>
where
    Message: 'a,
{
    fn from(breadcrumb: Breadcrumb<'a, Message>) -> Self {
        let last = breadcrumb.crumbs.len().saturating_sub(1);
        let size = breadcrumb.size;

        let items =
            breadcrumb
                .crumbs
                .iter()
                .enumerate()
                .flat_map(move |(index, crumb)| {
                    let color = if index == last {
                        color::TEXT_DEFAULT
                    } else {
                        color::TEXT_MUTED
                    };

                    let crumb: Element<'a, Message> =
                        text(*crumb).size(size).color(color).into();

                    if index == 0 {
                        vec![crumb]
                    } else {
                        let sep: Element<'a, Message> =
                            text("/").size(size).color(color::TEXT_MUTED).into();
                        vec![sep, crumb]
                    }
                });

        Row::with_children(items)
            .spacing(size.0 * 0.5)
            .align_y(Center)
            .into()
    }
}
