//! Shared eased hover animation primitive.
//!
//! Used by widgets that need a 0..1 factor that fades in and out as the cursor
//! enters and leaves them. Mid-flight reversal continues from where the
//! previous animation left off, so a quick in-out-in does not snap.

use std::time::{Duration, Instant};

use crate::easing;

/// How long every hover animation in the design system takes to fade in or
/// out.
pub const FADE: Duration = Duration::from_millis(100);

/// Below this factor a hover effect counts as fully hidden. Widgets gate their
/// hover draw/event paths on this to avoid forwarding to invisible children.
pub const EPSILON: f32 = 0.001;

/// The eased hover animation for one region. [`Hover::current`] reads the
/// live factor and [`Hover::flip`] retargets without snapping. Storing
/// `from`/`target`/`started` lets a mid-flight reversal continue from where
/// the previous one left off.
#[derive(Clone, Copy)]
pub struct Hover {
    from: f32,
    target: f32,
    started: Instant,
}

impl Default for Hover {
    fn default() -> Self {
        Self {
            from: 0.0,
            target: 0.0,
            started: Instant::now() - FADE,
        }
    }
}

impl Hover {
    /// The factor right now, eased from `from` toward `target` over [`FADE`].
    /// Fade-in uses an emphasized decelerate (fast start, soft settle).
    /// Fade-out uses the matching accelerate curve.
    pub fn current(&self, now: Instant) -> f32 {
        let raw = (now.duration_since(self.started).as_secs_f32() / FADE.as_secs_f32())
            .clamp(0.0, 1.0);
        let curve = if self.target >= self.from {
            &easing::EMPHASIZED_DECELERATE
        } else {
            &easing::EMPHASIZED_ACCELERATE
        };
        let eased = curve.y_at_x(raw);
        self.from + (self.target - self.from) * eased
    }

    /// Retargets to 1.0 if hovering, else 0.0. The new animation starts from
    /// the live factor so a reversal mid-flight is smooth. Returns whether
    /// the target changed, so callers can request a redraw on transition
    /// edges without keeping a parallel boolean of the last-known hover
    /// state. Idempotent when the target is unchanged.
    pub fn flip(&mut self, hovering: bool, now: Instant) -> bool {
        let target = if hovering { 1.0 } else { 0.0 };
        if target == self.target {
            return false;
        }
        self.from = self.current(now);
        self.target = target;
        self.started = now;
        true
    }

    /// Whether the region still has movement left.
    pub fn animating(&self, now: Instant) -> bool {
        now.duration_since(self.started) < FADE
    }

    /// A track already settled at its terminal value with no animation in
    /// flight. Used by externally-driven tracks (e.g. `selected`) so a
    /// pre-set value at mount time does not fade in from zero on the first
    /// frame.
    pub fn settled(at: bool) -> Self {
        let target = if at { 1.0 } else { 0.0 };
        Self {
            from: target,
            target,
            started: Instant::now() - FADE,
        }
    }
}

/// Shared hover and press bookkeeping for clickable widgets in the design
/// system. Owns the `hover` track and the `pressed` latch and offers the
/// reconciliation hooks every press-dispatching widget needs.
#[derive(Clone, Copy, Default)]
pub struct PressState {
    pub hover: Hover,
    /// True between a left-button press over bounds and its matching
    /// release. Cleared on any release we observe, even captured ones,
    /// so a release intercepted by a child cannot leak `pressed = true`
    /// into a later release-over-bounds and spuriously dispatch.
    pub pressed: bool,
}

impl PressState {
    /// Reconciles the hover track against the live cursor. Returns true if
    /// the track retargeted, so the caller can request a single redraw on
    /// the transition edge.
    pub fn reconcile(&mut self, over: bool, now: Instant) -> bool {
        self.hover.flip(over, now)
    }

    /// Records a left-button press at the cursor. Returns true if the
    /// press landed inside our bounds.
    pub fn press(&mut self, over: bool) -> bool {
        if over {
            self.pressed = true;
        }
        over
    }

    /// Records a left-button release. No-op when not in a press cycle.
    /// Otherwise clears `pressed` (even when the event is captured by a
    /// child, so the latch cannot leak across clicks). Returns true if the
    /// release completes a press cycle inside our bounds and the caller
    /// should dispatch.
    pub fn release(&mut self, over: bool) -> bool {
        if !self.pressed {
            return false;
        }
        self.pressed = false;
        over
    }

    pub fn animating(&self, now: Instant) -> bool {
        self.hover.animating(now)
    }
}
