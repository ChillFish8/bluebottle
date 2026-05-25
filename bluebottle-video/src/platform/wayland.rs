use std::ffi::{c_char, c_void};

use ash::vk;
use ash::vk::Handle;
use placebo_sys as pl;

use super::Surface;
use crate::error::Error;
use crate::placebo::vulkan::Instance;

/// libplacebo's `vkGetInstanceProcAddr` signature, unwrapped from its `Option`.
/// ABI-identical to `ash`'s `PFN_vkGetInstanceProcAddr`; the two differ only in
/// type names and the (no-op on this target) `"C"`/`"system"` convention.
type PlGetInstanceProcAddr = unsafe extern "C" fn(
    pl::VkInstance,
    *const c_char,
) -> Option<unsafe extern "C" fn()>;

/// Instance extensions required to present to a Wayland surface.
pub const EXTENSIONS: &[&str] = &["VK_KHR_surface", "VK_KHR_wayland_surface"];

/// Build a [`Surface`] for `(wl_display, wl_surface)` against `instance`.
///
/// libplacebo created the `VkInstance`, so we load the surface entry points
/// through its `get_proc_addr` (via `ash`) rather than the directly-linked
/// loader. The returned surface destroys itself with `vkDestroySurfaceKHR`.
pub fn create_surface(
    instance: &Instance,
    display: *mut c_void,
    surface: *mut c_void,
) -> Result<Surface, Error> {
    let get_proc_addr = instance.get_proc_addr().ok_or_else(|| Error::Surface {
        message: "instance has no vkGetInstanceProcAddr".into(),
    })?;

    // SAFETY: both function pointers have identical ABI (pointer-sized C
    // function pointers); only the wrapper type (Option, "C"/"system") differs.
    let static_fn = ash::StaticFn {
        get_instance_proc_addr: unsafe {
            std::mem::transmute::<PlGetInstanceProcAddr, vk::PFN_vkGetInstanceProcAddr>(
                get_proc_addr,
            )
        },
    };

    // SAFETY: `get_proc_addr` is the loader libplacebo built the instance with.
    let entry = unsafe { ash::Entry::from_static_fn(static_fn.clone()) };
    let instance_handle = vk::Instance::from_raw(instance.handle() as usize as u64);
    // SAFETY: `instance_handle` is the live `VkInstance` libplacebo owns; we only
    // load function pointers, never destroy it.
    let ash_instance = unsafe { ash::Instance::load(&static_fn, instance_handle) };
    let wayland = ash::khr::wayland_surface::Instance::new(&entry, &ash_instance);
    let surface_loader = ash::khr::surface::Instance::new(&entry, &ash_instance);

    let info = vk::WaylandSurfaceCreateInfoKHR::default()
        .display(display.cast())
        .surface(surface.cast());

    // SAFETY: the display/surface pointers are the live Wayland objects
    // bluebottle-window owns for the window's lifetime.
    let created = unsafe { wayland.create_wayland_surface(&info, None) };
    let vk_surface = created.map_err(|err| Error::Surface {
        message: format!("vkCreateWaylandSurfaceKHR failed: {err}"),
    })?;

    // `pl::VkSurfaceKHR` is a non-dispatchable handle (a pointer); `ash`'s is a
    // 64-bit `as_raw()`. Bridge by pointer/integer cast.
    let handle = vk_surface.as_raw() as usize as pl::VkSurfaceKHR;

    // The loader owns copies of the function pointers, so it can outlive `entry`
    // and `ash_instance` inside the destructor closure.
    let destroy = Box::new(move |handle: pl::VkSurfaceKHR| {
        let raw = vk::SurfaceKHR::from_raw(handle as u64);
        // SAFETY: destroyed once, after the swapchain and before libplacebo
        // destroys the owning VkInstance.
        unsafe { surface_loader.destroy_surface(raw, None) };
    });

    Ok(Surface::new(handle, destroy))
}
