use std::ffi::{CString, c_char};

use placebo_sys as pl;

use super::log::Log;
use crate::error::{CreateSnafu, Error};

/// A `Copy` handle to a libplacebo `pl_gpu`, valid for the lifetime of the
/// [`Device`] it came from. The newtype keeps the raw pointer out of the safe
/// API's signatures.
#[derive(Clone, Copy)]
pub struct Gpu(pl::pl_gpu);

impl Gpu {
    pub fn raw(self) -> pl::pl_gpu {
        self.0
    }
}

/// A libplacebo-managed `VkInstance` with caller-requested extensions.
///
/// Must be dropped after the [`Device`] created from it, and the [`Log`] must
/// outlive both; the owning render context guarantees this by field order.
pub struct Instance {
    raw: pl::pl_vk_inst,
}

impl Instance {
    /// Create an instance enabling `extensions` (e.g. `VK_KHR_surface`,
    /// `VK_KHR_wayland_surface`).
    pub fn new(log: &Log, extensions: &[&str]) -> Result<Self, Error> {
        let names: Vec<CString> = extensions
            .iter()
            .map(|ext| CString::new(*ext).expect("extension name has no NUL"))
            .collect();
        let ptrs: Vec<*const c_char> = names.iter().map(|name| name.as_ptr()).collect();

        let mut params = unsafe { pl::pl_vk_inst_default_params };
        params.extensions = ptrs.as_ptr();
        params.num_extensions = ptrs.len() as i32;

        let raw = unsafe { pl::pl_vk_inst_create(log.raw(), &params) };
        snafu::ensure!(!raw.is_null(), CreateSnafu { what: "pl_vk_inst" });

        Ok(Self { raw })
    }

    pub fn handle(&self) -> pl::VkInstance {
        unsafe { (*self.raw).instance }
    }

    pub fn get_proc_addr(&self) -> pl::PFN_vkGetInstanceProcAddr {
        unsafe { (*self.raw).get_proc_addr }
    }
}

impl Drop for Instance {
    fn drop(&mut self) {
        unsafe { pl::pl_vk_inst_destroy(&mut self.raw) };
    }
}

/// A libplacebo Vulkan device wrapping a `pl_gpu`.
///
/// Either headless (used by tests) or bound to a presentation surface, in which
/// case it picks a physical device that can present to that surface.
pub struct Device {
    raw: pl::pl_vulkan,
}

impl Device {
    pub fn headless(log: &Log) -> Result<Self, Error> {
        // A NULL `instance` makes libplacebo create its own internally.
        let params = unsafe { pl::pl_vulkan_default_params };
        Self::create(log, &params)
    }

    pub fn for_surface(
        log: &Log,
        instance: &Instance,
        surface: pl::VkSurfaceKHR,
    ) -> Result<Self, Error> {
        let mut params = unsafe { pl::pl_vulkan_default_params };
        params.instance = instance.handle();
        params.get_proc_addr = instance.get_proc_addr();
        params.surface = surface;
        Self::create(log, &params)
    }

    fn create(log: &Log, params: &pl::pl_vulkan_params) -> Result<Self, Error> {
        let raw = unsafe { pl::pl_vulkan_create(log.raw(), params) };
        snafu::ensure!(!raw.is_null(), CreateSnafu { what: "pl_vulkan" });
        Ok(Self { raw })
    }

    /// The abstract GPU used to allocate textures and run the renderer.
    pub fn gpu(&self) -> Gpu {
        Gpu(unsafe { (*self.raw).gpu })
    }

    /// The raw `pl_vulkan`, needed to create a swapchain.
    pub fn raw(&self) -> pl::pl_vulkan {
        self.raw
    }
}

impl Drop for Device {
    fn drop(&mut self) {
        unsafe { pl::pl_vulkan_destroy(&mut self.raw) };
    }
}
