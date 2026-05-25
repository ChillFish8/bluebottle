//! Platform-specific seam for video rendering.
//!
//! Everything above this module (the libplacebo wrappers, the renderer, the
//! GStreamer sink) is platform-neutral. Only two things differ per platform and
//! live here: creating the `VkSurfaceKHR` libplacebo presents onto, and the set
//! of Vulkan instance extensions that requires. Linux/Wayland is implemented;
//! Windows and macOS are stubs to grow into.

use std::ffi::c_void;

use placebo_sys as pl;

use crate::error::Error;
use crate::placebo::vulkan::Instance;

#[cfg(target_os = "linux")]
mod wayland;
#[cfg(target_os = "linux")]
use wayland as backend;

#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "macos")]
use macos as backend;

#[cfg(target_os = "windows")]
mod windows;
#[cfg(target_os = "windows")]
use windows as backend;

/// An owned `VkSurfaceKHR` that destroys itself on drop.
///
/// libplacebo's swapchain borrows the surface but does not own it, so we keep it
/// alive for the swapchain's lifetime and tear it down afterwards (the owning
/// render context drops the swapchain first, then this). The destructor is a
/// platform-provided closure capturing whatever loader it needs.
pub struct Surface {
    handle: pl::VkSurfaceKHR,
    destroy: Box<dyn FnMut(pl::VkSurfaceKHR)>,
}

impl Surface {
    /// Build a surface from its handle and a destructor.
    pub(crate) fn new(
        handle: pl::VkSurfaceKHR,
        destroy: Box<dyn FnMut(pl::VkSurfaceKHR)>,
    ) -> Self {
        Self { handle, destroy }
    }

    /// The raw `VkSurfaceKHR` to hand to libplacebo.
    pub fn handle(&self) -> pl::VkSurfaceKHR {
        self.handle
    }
}

impl Drop for Surface {
    fn drop(&mut self) {
        (self.destroy)(self.handle);
    }
}

/// The Vulkan instance extensions the current platform's surface needs.
pub fn surface_extensions() -> &'static [&'static str] {
    #[cfg(any(target_os = "linux", target_os = "macos", target_os = "windows"))]
    {
        backend::EXTENSIONS
    }
    #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
    {
        &[]
    }
}

/// Create a [`Surface`] from native display/surface handles.
///
/// On Wayland these are the `wl_display` and `wl_surface` pointers exposed by
/// `bluebottle-window`. On unsupported platforms this returns
/// [`Error::UnsupportedPlatform`].
pub fn create_surface(
    instance: &Instance,
    display: *mut c_void,
    surface: *mut c_void,
) -> Result<Surface, Error> {
    #[cfg(any(target_os = "linux", target_os = "macos", target_os = "windows"))]
    {
        backend::create_surface(instance, display, surface)
    }
    #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
    {
        let _ = (instance, display, surface);
        Err(Error::UnsupportedPlatform)
    }
}
