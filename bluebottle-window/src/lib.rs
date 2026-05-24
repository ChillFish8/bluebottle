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
mod wayland;

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
    // Render with Vulkan. The wgpu GLES/EGL backend drives the Wayland
    // display's default event queue, which deadlocks against the connection we
    // own on the loop thread; Vulkan WSI uses its own queue and composes
    // cleanly. iced only exposes the backend choice via this env var, which we
    // set if the caller has not already chosen one.
    if std::env::var_os("WGPU_BACKEND").is_none() {
        // SAFETY: set before any wgpu or thread initialisation in this call.
        unsafe { std::env::set_var("WGPU_BACKEND", "vulkan") };
    }

    wayland::run(build)
}
