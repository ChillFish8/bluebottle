//! Render an Iced application into a Wayland subsurface laid over a main
//! surface that this library creates and owns.
//!
//! The intended use is to overlay an Iced UI on top of content rendered by
//! something else (for example video drawn into a content subsurface by a sink).
//! Because Wayland has no foreign-surface embedding (no `--wid` equivalent),
//! ownership is flipped: this library owns every surface. It paints an opaque
//! black backdrop on the main surface, lays a transparent overlay subsurface over
//! it for the Iced UI, and in video mode adds a content subsurface beneath the
//! overlay for a sink. The caller drives the UI and, in video mode, the sink.
//!
//! Only Linux/Wayland is supported for now.

mod error;
mod handle;
mod overlay;
pub mod platform;

#[cfg(feature = "splash")]
pub use bluebottle_splash::Splash;
pub use error::Error;
pub use handle::Window;
pub use iced;
use iced::Program;

/// Pin the renderer to Vulkan and run the overlay, with an optional splash.
///
/// The splash argument is a platform-level type that carries a [`Splash`] only
/// when the `splash` feature is on and is otherwise an empty marker, so the
/// non-splash constructors stay identical across the feature.
fn run_overlay<P, F>(
    build: F,
    video: bool,
    splash: platform::SplashArg,
) -> Result<Window, Error>
where
    F: FnOnce() -> P + Send + 'static,
    P: Program + 'static,
{
    force_vulkan_backend();
    platform::run(build, video, splash)
}

/// Create an overlay window with full control over video mode and the splash.
///
/// `build` constructs the [`Program`] (see [`create_overlay`]). `video` adds the
/// content subsurface of [`create_video_overlay`] for a sink, beneath the UI.
/// `splash` shows the fading startup splash of [`create_overlay_with_splash`] on
/// its own top subsurface. The two combine, so a video app can also show a splash
/// with `create(build, true, Some(splash))`. Available with the `splash` feature;
/// the other constructors cover the cases that need no [`Splash`].
#[cfg(feature = "splash")]
pub fn create<P, F>(
    build: F,
    video: bool,
    splash: Option<Splash>,
) -> Result<Window, Error>
where
    F: FnOnce() -> P + Send + 'static,
    P: Program + 'static,
{
    run_overlay(build, video, splash)
}

/// Create an overlay window driven by an Iced application.
///
/// `build` constructs the [`Program`] — typically the value produced by
/// [`iced::application`] (and its builder methods) that you would otherwise
/// `.run()`, e.g. `|| iced::application(..)`. The program is built on the
/// dedicated render thread (the high-level Iced builder is not `Send`), so only
/// the closure itself crosses the thread boundary.
///
/// The Wayland event and render loop runs on that background thread. This
/// function returns once the window is ready. The library owns and paints the
/// main surface, so the caller only drives the overlay UI through the program.
pub fn create_overlay<P, F>(build: F) -> Result<Window, Error>
where
    F: FnOnce() -> P + Send + 'static,
    P: Program + 'static,
{
    run_overlay(build, false, Default::default())
}

/// Create an overlay window that shows an animated startup splash.
///
/// Like [`create_overlay`], but `bluebottle-window` paints `splash` (a logo over a
/// background) on the main surface while the overlay builds its first frame, then
/// hands the surface back once the UI is on screen. Available with the `splash`
/// feature.
#[cfg(feature = "splash")]
pub fn create_overlay_with_splash<P, F>(
    build: F,
    splash: Splash,
) -> Result<Window, Error>
where
    F: FnOnce() -> P + Send + 'static,
    P: Program + 'static,
{
    run_overlay(build, false, Some(splash))
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
    run_overlay(build, true, Default::default())
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
