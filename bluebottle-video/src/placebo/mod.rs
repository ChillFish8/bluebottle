//! Safe RAII wrappers over `placebo-sys`.
//!
//! Each type owns exactly one libplacebo object and destroys it on drop. The
//! wrappers are platform-neutral; surface creation (the one platform-specific
//! input to [`vulkan::Device::for_surface`]) is produced by [`crate::platform`].

pub mod frame;
pub mod log;
pub mod renderer;
pub mod swapchain;
pub mod vulkan;

pub use frame::{
    DmabufPlane,
    SysmemUploader,
    Texture,
    import_dmabuf,
    packed8_plane_data,
};
pub use log::Log;
pub use renderer::Renderer;
pub use swapchain::Swapchain;
pub use vulkan::{Device, Gpu, Instance};
