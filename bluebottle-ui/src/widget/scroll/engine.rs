//! The offset state machine shared by every vertical scroll widget.
//!
//! Owns the live offset, the bar's visual state, and the active
//! programmatic-scroll animation. Layout and any concept of an anchor live
//! with the parent widget. The engine just exposes the primitives a parent
//! needs to drive an offset cleanly.

use std::time::{Duration, Instant};

use crate::easing;
use crate::widget::scroll::bar::{BarEvent, ScrollBar};

/// How long an animated scroll-to-target takes.
const TARGET_DURATION: Duration = Duration::from_millis(320);

#[derive(Clone, Copy)]
struct Target {
    from: f32,
    started: Instant,
}

#[derive(Clone, Copy, Default)]
pub(crate) struct ScrollEngine {
    pub offset: f32,
    pub bar: ScrollBar,
    target: Option<Target>,
}

impl ScrollEngine {
    /// Begin an animated programmatic scroll from the current offset. The
    /// end point is resolved per frame by the parent and passed to
    /// [`Self::step_target`].
    pub fn start_target(&mut self, now: Instant) {
        self.target = Some(Target {
            from: self.offset,
            started: now,
        });
    }

    pub fn clear_target(&mut self) {
        self.target = None;
    }

    pub fn has_target(&self) -> bool {
        self.target.is_some()
    }

    /// True once the target has been alive longer than its animation
    /// duration. Lets parents bail out of stuck targets that never resolve
    /// to a valid end point.
    pub fn target_expired(&self, now: Instant) -> bool {
        self.target
            .is_some_and(|t| now.duration_since(t.started) > TARGET_DURATION)
    }

    /// Advances the active target animation toward `to` (the parent's
    /// freshly-resolved end point). Returns whether an animation is still
    /// in flight. A no-op when no target is active.
    pub fn step_target(&mut self, now: Instant, to: f32, max_offset: f32) -> bool {
        let Some(target) = self.target else {
            return false;
        };

        let raw = (now.duration_since(target.started).as_secs_f32()
            / TARGET_DURATION.as_secs_f32())
        .clamp(0.0, 1.0);

        let eased = easing::EMPHASIZED.y_at_x(raw);
        let to = to.clamp(0.0, max_offset);

        self.offset = (target.from + (to - target.from) * eased).clamp(0.0, max_offset);

        if raw >= 1.0 {
            self.target = None;
            false
        } else {
            true
        }
    }

    /// Applies the bar's reported event to the offset. Returns whether the
    /// user moved the offset, so the parent knows to emit `on_scroll` and
    /// recompute derived state.
    pub fn apply_bar_event(
        &mut self,
        event: BarEvent,
        max_offset: f32,
        now: Instant,
    ) -> bool {
        let prev = self.offset;

        match event {
            BarEvent::Wheel(dy) => {
                self.offset = (self.offset - dy).clamp(0.0, max_offset);
            },
            BarEvent::DragTo(off) => {
                self.offset = off.clamp(0.0, max_offset);
            },
            BarEvent::Captured | BarEvent::None => return false,
        }

        if self.offset == prev {
            return false;
        }

        // User input always wins over a pending programmatic scroll.
        self.target = None;
        self.bar.note_scrolled(now);
        true
    }

    /// Shifts the offset by `delta`, used by parents that maintain a
    /// content anchor across reflows. Suppressed while a target animation
    /// is active so the engine does not fight the resolved end point.
    pub fn shift_offset(&mut self, delta: f32, max_offset: f32) {
        if self.target.is_some() || delta == 0.0 {
            return;
        }
        self.offset = (self.offset + delta).clamp(0.0, max_offset);
    }

    /// Clamps the offset into the current scroll range, useful after a
    /// reflow shrinks the content.
    pub fn clamp(&mut self, max_offset: f32) {
        self.offset = self.offset.clamp(0.0, max_offset);
    }

    /// True while the bar or a target animation is in flight.
    pub fn animating(&self, now: Instant) -> bool {
        self.target.is_some() || self.bar.animating(now)
    }
}
