use iced::advanced::Widget;
use iced::{Length, Size};

/// Returns the size of the widget with the set iced theme and renderer.
pub fn widget_size<M>(
    widget: &impl Widget<M, iced::Theme, iced::Renderer>,
) -> Size<Length> {
    widget.size()
}
