use std::time::Duration;

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

/// Short human duration. Rounds partial minutes up so the last 59 seconds of
/// a film read as `1m` rather than collapsing to `0m`. Returns `0m` for an
/// empty duration so the slot still occupies space.
pub fn format_duration_short(d: Duration) -> String {
    let seconds = d.as_secs();
    if seconds == 0 {
        return "0m".into();
    }

    let total_minutes = seconds.div_ceil(60);
    let hours = total_minutes / 60;
    let minutes = total_minutes % 60;

    match (hours, minutes) {
        (0, m) => format!("{m}m"),
        (h, 0) => format!("{h}h"),
        (h, m) => format!("{h}h {m}m"),
    }
}
