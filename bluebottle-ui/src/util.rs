use iced::advanced::Widget;
use iced::{Length, Size};

/// Returns the size of the widget with the set iced theme and renderer.
pub fn widget_size<M>(
    widget: &impl Widget<M, iced::Theme, iced::Renderer>,
) -> Size<Length> {
    widget.size()
}

/// Linear interpolation from `from` to `to` by `t` in 0..1.
pub fn lerp(from: f32, to: f32, t: f32) -> f32 {
    from + (to - from) * t
}
