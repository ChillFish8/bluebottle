//! Spacing scale shared across the crate. One numeric ladder, two semantic
//! aliases. `GAP_*` is for `.spacing(...)` on rows and columns, `PAD_*` is for
//! `.padding(...)`. Both groups resolve to the same values, and the number in
//! the name is the pixel value.

pub const GAP_2: f32 = 2.0;
pub const GAP_4: f32 = 4.0;
pub const GAP_6: f32 = 6.0;
pub const GAP_8: f32 = 8.0;
pub const GAP_10: f32 = 10.0;
pub const GAP_12: f32 = 12.0;
pub const GAP_14: f32 = 14.0;
pub const GAP_16: f32 = 16.0;
pub const GAP_20: f32 = 20.0;
pub const GAP_24: f32 = 24.0;
pub const GAP_32: f32 = 32.0;
pub const GAP_40: f32 = 40.0;

// PAD_* aliases GAP_* on the same scale.
pub const PAD_2: f32 = GAP_2;
pub const PAD_4: f32 = GAP_4;
pub const PAD_6: f32 = GAP_6;
pub const PAD_8: f32 = GAP_8;
pub const PAD_10: f32 = GAP_10;
pub const PAD_12: f32 = GAP_12;
pub const PAD_14: f32 = GAP_14;
pub const PAD_16: f32 = GAP_16;
pub const PAD_20: f32 = GAP_20;
pub const PAD_24: f32 = GAP_24;
pub const PAD_32: f32 = GAP_32;
pub const PAD_40: f32 = GAP_40;
