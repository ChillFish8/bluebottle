use std::ffi::{CString, c_char};

use placebo_sys as pl;

use super::log::Log;
use crate::error::{CreateSnafu, Error};

/// A handle to a libplacebo `pl_gpu`, wrapping the raw pointer in a `Copy`
/// newtype so it can be passed around the safe API without tripping raw-pointer
/// lints. The GPU is owned by the [`Device`] it came from and is only valid for
/// that device's lifetime.
#[derive(Clone, Copy)]
pub struct Gpu(pl::pl_gpu);

impl Gpu {
    /// The underlying `pl_gpu` handle.
    pub fn raw(self) -> pl::pl_gpu {
        self.0
    }
}

/// A libplacebo-managed `VkInstance` with caller-requested extensions.
///
/// We let libplacebo own instance creation (it wires up the debug callback to
/// the [`Log`]) but request the windowing-system surface extensions ourselves so
/// the platform layer can build a `VkSurfaceKHR` against it.
///
/// Must be dropped after the [`Device`] created from it; the owning render
/// context guarantees this by field order, and the [`Log`] must outlive both.
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

        // SAFETY: reading a libplacebo `extern const` defaults struct (Copy).
        let mut params = unsafe { pl::pl_vk_inst_default_params };
        params.extensions = ptrs.as_ptr();
        params.num_extensions = ptrs.len() as i32;

        // SAFETY: `params` (and the arrays it points at) outlive the call;
        // libplacebo copies what it needs.
        let raw = unsafe { pl::pl_vk_inst_create(log.raw(), &params) };
        snafu::ensure!(!raw.is_null(), CreateSnafu { what: "pl_vk_inst" });
        Ok(Self { raw })
    }

    /// The underlying `VkInstance` handle.
    pub fn handle(&self) -> pl::VkInstance {
        // SAFETY: `raw` is a valid `pl_vk_inst` for the lifetime of `self`.
        unsafe { (*self.raw).instance }
    }

    /// The `vkGetInstanceProcAddr` libplacebo loaded the instance with.
    pub fn get_proc_addr(&self) -> pl::PFN_vkGetInstanceProcAddr {
        // SAFETY: as above.
        unsafe { (*self.raw).get_proc_addr }
    }
}

impl Drop for Instance {
    fn drop(&mut self) {
        // SAFETY: created by `pl_vk_inst_create`, destroyed once.
        unsafe { pl::pl_vk_inst_destroy(&mut self.raw) };
    }
}

/// A libplacebo Vulkan device wrapping a `pl_gpu`.
///
/// Either headless (libplacebo creates its own instance — used by tests) or
/// bound to a presentation surface, in which case it picks a physical device
/// that can present to that surface.
pub struct Device {
    raw: pl::pl_vulkan,
}

impl Device {
    /// Create a headless device on the "best" available GPU.
    pub fn headless(log: &Log) -> Result<Self, Error> {
        // SAFETY: reading libplacebo's `extern const` defaults (Copy). A NULL
        // `instance` makes libplacebo create its own internally.
        let params = unsafe { pl::pl_vulkan_default_params };
        Self::create(log, &params)
    }

    /// Create a device able to present to `surface`, using `instance`.
    pub fn for_surface(
        log: &Log,
        instance: &Instance,
        surface: pl::VkSurfaceKHR,
    ) -> Result<Self, Error> {
        // SAFETY: reading libplacebo's `extern const` defaults (Copy).
        let mut params = unsafe { pl::pl_vulkan_default_params };
        params.instance = instance.handle();
        params.get_proc_addr = instance.get_proc_addr();
        params.surface = surface;
        Self::create(log, &params)
    }

    fn create(log: &Log, params: &pl::pl_vulkan_params) -> Result<Self, Error> {
        // SAFETY: `params` outlives the call; libplacebo copies what it needs.
        let raw = unsafe { pl::pl_vulkan_create(log.raw(), params) };
        snafu::ensure!(!raw.is_null(), CreateSnafu { what: "pl_vulkan" });
        Ok(Self { raw })
    }

    /// The abstract GPU used to allocate textures and run the renderer.
    pub fn gpu(&self) -> Gpu {
        // SAFETY: `raw` is valid for the lifetime of `self`.
        Gpu(unsafe { (*self.raw).gpu })
    }

    /// The raw `pl_vulkan`, needed to create a swapchain.
    pub fn raw(&self) -> pl::pl_vulkan {
        self.raw
    }
}

impl Drop for Device {
    fn drop(&mut self) {
        // SAFETY: created by `pl_vulkan_create`, destroyed once. All GPU objects
        // derived from it (textures, renderer, swapchain) are dropped first by
        // construction (see the owning render context's field order).
        unsafe { pl::pl_vulkan_destroy(&mut self.raw) };
    }
}
