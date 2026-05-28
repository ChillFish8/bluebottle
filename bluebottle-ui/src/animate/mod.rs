//! Reusable animation primitives. Each module exposes a small piece of state
//! (a factor, a phase, an instant) plus the easing/timing logic that drives
//! it, so widgets can compose hover, focus, press, and selection animations
//! without re-implementing the timing loop.

pub mod hover;
