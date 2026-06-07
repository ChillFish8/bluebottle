//! Shared infrastructure for shader widgets that lean on a separable Gaussian
//! blur. The WGSL lives in `shader.wgsl`; `gpu` and `pipeline` build the wgpu
//! resources around it.

pub mod backdrop;
pub mod gpu;
pub mod pipeline;

pub use backdrop::Backdrop;
