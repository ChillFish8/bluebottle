//! Render an Iced application into a Wayland subsurface laid over a main
//! surface that this library creates and owns.
//!
//! The intended use is to overlay an Iced UI on top of content rendered by
//! something else (for example video drawn into the main surface via libmpv's
//! render API). Because Wayland has no foreign-surface embedding (no `--wid`
//! equivalent), ownership is flipped: this library owns every surface, creates
//! a transparent overlay subsurface for the Iced UI, and hands the caller a
//! [`Window`] handle to the main surface to render into however they like.
//!
//! Only Linux/Wayland is supported for now.

mod error;
mod handle;
mod overlay;
pub mod platform;

pub use error::Error;
pub use handle::Window;
/// Re-export of the `iced` version this crate is built against.
pub use iced;
use iced::Program;

/// Create an overlay window driven by an Iced application.
///
/// `build` constructs the [`iced::Program`] — typically the value produced by
/// [`iced::application`] (and its builder methods) that you would otherwise
/// `.run()`, e.g. `|| iced::application(..)`. The program is built on the
/// dedicated render thread (the high-level Iced builder is not `Send`), so only
/// the closure itself crosses the thread boundary.
///
/// The Wayland event and render loop runs on that background thread; this
/// function returns as soon as the main surface is ready, so the caller can
/// immediately start rendering into it via the returned [`Window`].
pub fn create_overlay<P, F>(build: F) -> Result<Window, Error>
where
    F: FnOnce() -> P + Send + 'static,
    P: Program + 'static,
{
    force_vulkan_backend();
    platform::run(build, false)
}

/// Create an overlay window that also hosts an external video sink.
///
/// Behaves like [`create_overlay`], but the library additionally creates a
/// transparent *content* subsurface stacked beneath the Iced overlay and drives
/// the main surface itself (an opaque black backdrop). The content surface is
/// exposed through the platform extension trait (e.g.
/// `platform::wayland::WindowExtWayland::wl_video_surface_ptr`) so a video sink
/// — such as GStreamer's `waylandsink` via `GstVideoOverlay` — can render into
/// it directly, below the UI. Unlike [`create_overlay`], the caller does not
/// render the main surface.
pub fn create_video_overlay<P, F>(build: F) -> Result<Window, Error>
where
    F: FnOnce() -> P + Send + 'static,
    P: Program + 'static,
{
    force_vulkan_backend();
    platform::run(build, true)
}

/// Pin the Iced overlay renderer to wgpu's Vulkan backend.
///
/// The wgpu GLES/EGL backend drives the Wayland display's default event queue,
/// which deadlocks against the connection we own on the loop thread; Vulkan WSI
/// uses its own queue and composes cleanly. iced only exposes the backend choice
/// via this env var, which we set if the caller has not already chosen one.
fn force_vulkan_backend() {
    if std::env::var_os("WGPU_BACKEND").is_none() {
        // SAFETY: set before any wgpu or thread initialisation in this call.
        unsafe { std::env::set_var("WGPU_BACKEND", "vulkan") };
    }
}
