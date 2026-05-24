use std::ffi::c_void;
use std::ptr::NonNull;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;

use raw_window_handle::{WaylandDisplayHandle, WaylandWindowHandle};

use crate::error::Error;

/// Raw pointers to the main (parent) Wayland objects.
///
/// These are created on the event loop thread but handed to the caller, so the
/// wrapper is explicitly marked `Send`/`Sync`; the pointers themselves are only
/// dereferenced through the (thread-safe) Wayland C library.
#[derive(Clone, Copy)]
pub(crate) struct RawHandles {
    pub display: NonNull<c_void>,
    pub surface: NonNull<c_void>,
}

// SAFETY: the pointers reference `wl_display`/`wl_surface` objects owned for the
// lifetime of the loop thread; libwayland access is internally synchronised.
unsafe impl Send for RawHandles {}
unsafe impl Sync for RawHandles {}

/// State shared between the caller's [`Window`] handle and the event loop.
pub(crate) struct Shared {
    pub handles: RawHandles,
    pub size: Mutex<(u32, u32)>,
    pub scale: Mutex<f64>,
    pub close_requested: AtomicBool,
}

/// A handle to the main (parent) surface of an overlay window.
///
/// The Wayland event and render loop runs on a background thread; this handle
/// lets the caller read the surface geometry, obtain raw handles to render into
/// the main surface, and request shutdown. Dropping the handle requests close
/// and detaches from the loop; call [`Window::join`] to wait for a clean exit.
pub struct Window {
    shared: Arc<Shared>,
    thread: Option<JoinHandle<Result<(), Error>>>,
}

impl Window {
    pub(crate) fn new(
        shared: Arc<Shared>,
        thread: JoinHandle<Result<(), Error>>,
    ) -> Self {
        Self {
            shared,
            thread: Some(thread),
        }
    }

    /// Returns a pointer to the main surface's `wl_display`.
    pub fn wl_display_ptr(&self) -> *mut c_void {
        self.shared.handles.display.as_ptr()
    }

    /// Returns a pointer to the main `wl_surface`.
    pub fn main_surface_ptr(&self) -> *mut c_void {
        self.shared.handles.surface.as_ptr()
    }

    /// Returns a `raw-window-handle` handle for the main surface.
    ///
    /// Use this together with [`Window::raw_display_handle`] to build a graphics
    /// context (wgpu, EGL, libmpv's render API, ...) that draws into the main
    /// surface, beneath the overlay.
    pub fn raw_window_handle(&self) -> WaylandWindowHandle {
        WaylandWindowHandle::new(self.shared.handles.surface)
    }

    /// Returns a `raw-window-handle` handle for the Wayland display.
    pub fn raw_display_handle(&self) -> WaylandDisplayHandle {
        WaylandDisplayHandle::new(self.shared.handles.display)
    }

    /// Returns the current size of the window in physical pixels.
    pub fn size(&self) -> (u32, u32) {
        *self.shared.size.lock().expect("size mutex poisoned")
    }

    /// Returns the current scale factor reported by the compositor.
    pub fn scale_factor(&self) -> f64 {
        *self.shared.scale.lock().expect("scale mutex poisoned")
    }

    /// Requests that the overlay window close and the event loop exit.
    pub fn request_close(&self) {
        self.shared.close_requested.store(true, Ordering::Release);
    }

    /// Blocks until the event loop has exited, returning its result.
    pub fn join(mut self) -> Result<(), Error> {
        match self.thread.take() {
            Some(thread) => thread.join().unwrap_or(Ok(())),
            None => Ok(()),
        }
    }
}

impl Drop for Window {
    fn drop(&mut self) {
        self.request_close();
    }
}
