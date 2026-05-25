//! Windows backend stub.
//!
//! A real implementation would request the `VK_KHR_win32_surface` instance
//! extension and create the `VkSurfaceKHR` from an `HWND` (and import frames
//! via D3D11 shared handles). Not yet implemented — see [`super`] for the
//! platform-neutral seam this would plug into.

use std::ffi::c_void;

use super::Surface;
use crate::error::Error;
use crate::placebo::vulkan::Instance;

pub const EXTENSIONS: &[&str] = &[];

pub fn create_surface(
    _instance: &Instance,
    _display: *mut c_void,
    _surface: *mut c_void,
) -> Result<Surface, Error> {
    Err(Error::UnsupportedPlatform)
}
