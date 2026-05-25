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
    /// Create a FIFO (vsync) swapchain on `surface`.
    pub fn new(device: &Device, surface: pl::VkSurfaceKHR) -> Result<Self, Error> {
        let params = pl::pl_vulkan_swapchain_params {
            surface,
            // Default-init selects IMMEDIATE; FIFO is the safe vsync default.
            present_mode: pl::VkPresentModeKHR_VK_PRESENT_MODE_FIFO_KHR,
            ..Default::default()
        };
        // SAFETY: `params` outlives the call; `surface` belongs to the same
        // instance as `device`.
        let raw = unsafe { pl::pl_vulkan_create_swapchain(device.raw(), &params) };
        snafu::ensure!(
            !raw.is_null(),
            CreateSnafu {
                what: "pl_swapchain"
            }
        );
        Ok(Self { raw })
    }

    /// Resize the swapchain to `width`×`height` physical pixels.
    ///
    /// Returns the size libplacebo actually adopted (it may clamp). A `false`
    /// return from libplacebo (surface not ready) leaves the size unchanged.
    pub fn resize(&self, width: u32, height: u32) -> (u32, u32) {
        let mut w = width as i32;
        let mut h = height as i32;
        // SAFETY: in/out pointers are valid for the call.
        unsafe { pl::pl_swapchain_resize(self.raw, &mut w, &mut h) };
        (w.max(0) as u32, h.max(0) as u32)
    }

    /// Begin a frame, yielding the framebuffer to render into, or `None` if the
    /// surface is currently unavailable (hidden/minimised) and the frame should
    /// be skipped.
    pub fn start_frame(&self) -> Option<pl::pl_swapchain_frame> {
        let mut frame = pl::pl_swapchain_frame::default();
        // SAFETY: `frame` is valid out storage for the call.
        let ok = unsafe { pl::pl_swapchain_start_frame(self.raw, &mut frame) };
        ok.then_some(frame)
    }

    /// Submit the rendered frame. Returns `false` if submission failed.
    pub fn submit_frame(&self) -> bool {
        // SAFETY: paired with a successful `start_frame`.
        unsafe { pl::pl_swapchain_submit_frame(self.raw) }
    }

    /// Present the most recently submitted frame.
    pub fn swap_buffers(&self) {
        // SAFETY: paired with `submit_frame`.
        unsafe { pl::pl_swapchain_swap_buffers(self.raw) };
    }
}

impl Drop for Swapchain {
    fn drop(&mut self) {
        // SAFETY: created by `pl_vulkan_create_swapchain`, destroyed once,
        // before the device.
        unsafe { pl::pl_swapchain_destroy(&mut self.raw) };
    }
}
