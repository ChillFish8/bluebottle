//! `placebosink`: the libplacebo-backed GStreamer video sink.
//!
//! The element renders each frame with libplacebo and presents it via a Vulkan
//! swapchain onto a Wayland surface supplied by the application. Because
//! gstreamer-rs cannot implement the `GstVideoOverlay` interface from a Rust
//! subclass, the presentation target is supplied through the inherent methods
//! below ([`PlaceboSink::set_display`] / [`set_window_handle`] /
//! [`set_render_rectangle`]) rather than the overlay interface; the high-level
//! [`crate::Player`] drives them.
//!
//! [`set_window_handle`]: PlaceboSink::set_window_handle
//! [`set_render_rectangle`]: PlaceboSink::set_render_rectangle

use std::ffi::c_void;

use gst::glib;
use gst::prelude::*;
use gst::subclass::prelude::*;
use gstreamer as gst;

mod imp;

gst::glib::wrapper! {
    pub struct PlaceboSink(ObjectSubclass<imp::PlaceboSink>)
        @extends gstreamer_video::VideoSink, gstreamer_base::BaseSink, gst::Element, gst::Object;
}

impl PlaceboSink {
    /// Create a new, unparented sink instance.
    pub fn new() -> Self {
        glib::Object::new()
    }

    /// Set the `wl_display` the swapchain shares with the compositor.
    ///
    /// # Safety
    /// `display` must be a live `wl_display` pointer that outlives the sink's
    /// use of it (typically owned by `bluebottle-window`).
    pub unsafe fn set_display(&self, display: *mut c_void) {
        self.imp().set_display(display);
    }

    /// Set the `wl_surface` to present onto.
    ///
    /// # Safety
    /// `handle` must be a live `wl_surface` pointer that outlives the sink's use
    /// of it (typically the content surface from `bluebottle-window`).
    pub unsafe fn set_window_handle(&self, handle: *mut c_void) {
        self.imp().set_window_handle(handle);
    }

    /// Set the on-screen render size in logical pixels. Safe to call at any
    /// time; a change resizes the swapchain before the next frame.
    pub fn set_render_rectangle(&self, width: u32, height: u32) {
        self.imp().set_render_rectangle(width, height);
    }

    /// Set the render-quality preset. Safe to call at any time; takes effect on
    /// the next frame.
    pub fn set_render_preset(&self, preset: crate::config::RenderPreset) {
        self.imp().set_render_preset(preset);
    }

    /// A snapshot of the render path (format, zero-copy, preset, perf), or
    /// `None` until rendering has started.
    pub fn render_stats(&self) -> Option<crate::stats::RenderStats> {
        self.imp().render_stats()
    }
}

impl Default for PlaceboSink {
    fn default() -> Self {
        Self::new()
    }
}

/// Register `placebosink` so it can be created by name (e.g. via `playbin`'s
/// `video-sink`, `gst-launch`, or `ElementFactory`).
///
/// Pass `None` to register into the default registry for in-process use without
/// installing a plugin; the [`plugin_register_static!`] macro calls this with
/// the plugin when loaded as a shared object.
pub fn register(plugin: Option<&gst::Plugin>) -> Result<(), glib::BoolError> {
    gst::Element::register(
        plugin,
        "placebosink",
        gst::Rank::NONE,
        PlaceboSink::static_type(),
    )
}
