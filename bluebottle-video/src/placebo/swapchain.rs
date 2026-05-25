use placebo_sys as pl;

use super::vulkan::Device;
use crate::error::{CreateSnafu, Error};

/// A libplacebo Vulkan swapchain presenting onto a `VkSurfaceKHR`.
///
/// Must be dropped before the [`Device`] it was created from; the owning render
/// context guarantees this by field order.
pub struct Swapchain {
    raw: pl::pl_swapchain,
}

impl Swapchain {
    pub fn new(device: &Device, surface: pl::VkSurfaceKHR) -> Result<Self, Error> {
        let params = pl::pl_vulkan_swapchain_params {
            surface,
            // MAILBOX is vsync'd (tear-free) but latest-wins: it discards stale
            // queued frames instead of draining them one-per-refresh, so a burst
            // of resizes snaps to the final size rather than playing out every
            // intermediate one. libplacebo falls back to FIFO if it is
            // unsupported. (Zero-init would mean IMMEDIATE, which tears.)
            present_mode: pl::VkPresentModeKHR_VK_PRESENT_MODE_MAILBOX_KHR,
            ..Default::default()
        };

        let raw = unsafe { pl::pl_vulkan_create_swapchain(device.raw(), &params) };
        snafu::ensure!(
            !raw.is_null(),
            CreateSnafu {
                what: "pl_swapchain"
            }
        );

        Ok(Self { raw })
    }

    /// Resize the swapchain to `width`×`height` pixels.
    ///
    /// Returns whether libplacebo adopted the request; `false` (surface
    /// unavailable) leaves the size unchanged and the caller should retry.
    pub fn resize(&self, width: u32, height: u32) -> bool {
        let mut w = width as i32;
        let mut h = height as i32;
        unsafe { pl::pl_swapchain_resize(self.raw, &mut w, &mut h) }
    }

    /// Begin a frame, or `None` if the surface is currently unavailable
    /// (hidden/minimised) and the frame should be skipped.
    pub fn start_frame(&self) -> Option<pl::pl_swapchain_frame> {
        let mut frame = pl::pl_swapchain_frame::default();
        let ok = unsafe { pl::pl_swapchain_start_frame(self.raw, &mut frame) };
        ok.then_some(frame)
    }

    pub fn submit_frame(&self) -> bool {
        unsafe { pl::pl_swapchain_submit_frame(self.raw) }
    }

    pub fn swap_buffers(&self) {
        unsafe { pl::pl_swapchain_swap_buffers(self.raw) };
    }
}

impl Drop for Swapchain {
    fn drop(&mut self) {
        unsafe { pl::pl_swapchain_destroy(&mut self.raw) };
    }
}
