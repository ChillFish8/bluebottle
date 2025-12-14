use iced::widget::scrollable::{
    Anchor,
    Direction,
    Rail,
    Scrollbar,
    Scroller,
    Status,
    Style,
};
use iced::{Background, Element, Theme, border, widget};

use crate::color;

// TODO: Improve the behaviour of this with styling
/// Allows a container to become scrollable on an overflow.
pub fn scrollable<'a, Message>(
    content: impl Into<Element<'a, Message>>,
) -> widget::Scrollable<'a, Message> {
    fn style(theme: &Theme, status: Status) -> Style {
        let color = match status {
            Status::Active { .. } => color::HOVER_HIGHLIGHT,
            Status::Hovered { .. } => color::HOVER_HIGHLIGHT,
            Status::Dragged { .. } => color::HOVER_HIGHLIGHT,
        };

        let mut style = widget::scrollable::default(theme, status);
        style.vertical_rail = Rail {
            background: None,
            border: Default::default(),
            scroller: Scroller {
                background: Background::Color(color),
                border: border::rounded(8),
            },
        };
        style
    }

    let bar = Scrollbar::new()
        .scroller_width(4)
        .margin(0)
        .anchor(Anchor::Start);

    widget::scrollable(content)
        .direction(Direction::Vertical(bar))
        .style(style)
}
