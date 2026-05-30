//! Platform backends and their window extension traits.
//!
//! Each supported platform provides a backend module that owns the native
//! surfaces and event loop and drives the (platform-agnostic) Iced overlay in
//! [`crate::overlay`]. The backend for the current target is selected here and
//! re-exported as [`run`], so [`crate::create_overlay`] never names a platform.
//!
//! Platform-specific extensions to [`crate::Window`] (for example, access to
//! the underlying `wl_display`/`wl_surface` pointers) live in the corresponding
//! submodule's extension trait, mirroring `winit::platform`. Adding Windows or
//! macOS support means adding a sibling module here that exposes a `run` with
//! the same signature plus its own `WindowExt*` trait — nothing else in the
//! crate needs to change.

#[cfg(target_os = "linux")]
pub mod wayland;

#[cfg(target_os = "linux")]
pub(crate) use wayland::{SplashArg, run};

#[cfg(not(target_os = "linux"))]
compile_error!(
    "bluebottle-window currently only supports Linux/Wayland; Windows and \
     macOS backends are not yet implemented"
);
