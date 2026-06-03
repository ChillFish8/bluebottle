//! Helpers for partial reveal of a polyline on a canvas.
//!
//! Used by widgets that draw a stroked outline progressively as an animation
//! factor advances from 0 to 1. The route is treated as a polyline. The
//! traced sub-path includes every full segment up to the factor-scaled total
//! arc length, plus an interpolated stub on the final segment so the tip
//! lands mid-edge instead of snapping vertex-to-vertex.

use iced::Point;
use iced::widget::canvas;

/// Adds the leading `factor` fraction of `route`'s arc length to `builder` as
/// a fresh sub-path. The final segment is interpolated so the visible tip
/// lands at the right place mid-edge, not at the next vertex.
pub fn trace_partial(builder: &mut canvas::path::Builder, route: &[Point], factor: f32) {
    let total: f32 = route.windows(2).map(|w| distance(w[0], w[1])).sum();
    let target = factor.clamp(0.0, 1.0) * total;

    builder.move_to(route[0]);

    let mut walked = 0.0;
    for window in route.windows(2) {
        let segment = distance(window[0], window[1]);
        if walked + segment >= target {
            let t = if segment > 0.0 {
                (target - walked) / segment
            } else {
                0.0
            };
            builder.line_to(lerp(window[0], window[1], t));
            break;
        }
        builder.line_to(window[1]);
        walked += segment;
    }
}

/// Euclidean distance between two points.
pub fn distance(a: Point, b: Point) -> f32 {
    (a.x - b.x).hypot(a.y - b.y)
}

/// Linear interpolation between two points by `t` in 0..1.
pub fn lerp(a: Point, b: Point, t: f32) -> Point {
    Point::new(a.x + (b.x - a.x) * t, a.y + (b.y - a.y) * t)
}
