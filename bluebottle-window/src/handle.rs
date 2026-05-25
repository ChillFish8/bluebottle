use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;

use iced_runtime::core::input_method::InputMethod;
use iced_runtime::core::mouse;
use raw_window_handle::{
    DisplayHandle,
    HandleError,
    HasDisplayHandle,
    HasWindowHandle,
    RawDisplayHandle,
    RawWindowHandle,
    WindowHandle,
};

use crate::error::Error;

/// Raw handles to the main (parent) window objects, in platform-neutral form.
///
/// These are created on the event loop thread but handed to the caller, so the
/// wrapper is explicitly marked `Send`/`Sync`; the handles themselves are only
/// dereferenced through the (thread-safe) native windowing library.
#[derive(Clone, Copy)]
pub(crate) struct RawHandles {
    pub window: RawWindowHandle,
    pub display: RawDisplayHandle,
}

// SAFETY: the handles reference native window/display objects owned for the
// lifetime of the loop thread; access goes through libraries that synchronise
// internally (e.g. libwayland).
unsafe impl Send for RawHandles {}
unsafe impl Sync for RawHandles {}

/// State shared between the caller's [`Window`] handle and the event loop.
pub(crate) struct Shared {
    pub handles: RawHandles,
    pub size: Mutex<(u32, u32)>,
    pub scale: Mutex<f64>,
    pub close_requested: AtomicBool,
    /// Cursor the overlay wants shown, published by the render thread and
    /// applied by the event-loop thread (which owns the pointer).
    pub cursor: Mutex<mouse::Interaction>,
    /// Input-method state the overlay wants, published by the render thread and
    /// applied by the event-loop thread (which owns the text input).
    pub ime: Mutex<InputMethod>,
    /// Wakes the event-loop thread so it re-checks the cross-thread state above
    /// (and `close_requested`). The loop blocks indefinitely when idle, so any
    /// thread that mutates this state must call [`Shared::wake`] afterwards or
    /// the change is not observed until the next unrelated Wayland event. On
    /// Wayland this signals a calloop ping registered with the loop.
    pub wake: Arc<dyn Fn() + Send + Sync>,
}

impl Shared {
    /// Wake the event-loop thread to re-check cross-thread state.
    pub fn wake(&self) {
        (self.wake)();
    }
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

    /// Returns the raw window handle for the main (parent) surface.
    ///
    /// Use this together with [`Window::raw_display_handle`] to build a graphics
    /// context (wgpu, EGL, libmpv's render API, ...) that draws into the main
    /// surface, beneath the overlay. [`Window`] also implements
    /// [`HasWindowHandle`]/[`HasDisplayHandle`], so it can be passed directly to
    /// APIs that accept those. For platform-specific handles (e.g. the raw
    /// `wl_display`/`wl_surface` pointers) use the extension traits in
    /// [`crate::platform`].
    pub fn raw_window_handle(&self) -> RawWindowHandle {
        self.shared.handles.window
    }

    /// Returns the raw display handle for the main (parent) surface.
    pub fn raw_display_handle(&self) -> RawDisplayHandle {
        self.shared.handles.display
    }

    /// Returns the current size of the window in logical pixels.
    pub fn size(&self) -> (u32, u32) {
        *self.shared.size.lock().expect("size mutex poisoned")
    }

    /// Returns the current size of the window in physical pixels.
    ///
    /// This is the buffer size the caller should render the main surface at.
    pub fn physical_size(&self) -> (u32, u32) {
        let (width, height) = self.size();
        let scale = self.scale_factor();
        (
            ((width as f64) * scale).round() as u32,
            ((height as f64) * scale).round() as u32,
        )
    }

    /// Returns the current scale factor reported by the compositor.
    pub fn scale_factor(&self) -> f64 {
        *self.shared.scale.lock().expect("scale mutex poisoned")
    }

    /// Returns whether the window is still open.
    ///
    /// Becomes `false` after [`Window::request_close`] or once the compositor
    /// asks the window to close; callers can poll this to stop rendering.
    pub fn is_open(&self) -> bool {
        !self.shared.close_requested.load(Ordering::Acquire)
    }

    /// Requests that the overlay window close and the event loop exit.
    pub fn request_close(&self) {
        self.shared.close_requested.store(true, Ordering::Release);
        // The loop blocks indefinitely when idle; wake it so the close is
        // observed promptly rather than at the next unrelated event.
        self.shared.wake();
    }

    /// Blocks until the event loop has exited, returning its result.
    pub fn join(mut self) -> Result<(), Error> {
        match self.thread.take() {
            Some(thread) => thread.join().unwrap_or(Ok(())),
            None => Ok(()),
        }
    }
}

impl HasWindowHandle for Window {
    fn window_handle(&self) -> Result<WindowHandle<'_>, HandleError> {
        // SAFETY: the handle references the main surface, which the loop thread
        // keeps alive for at least as long as this `Window`.
        Ok(unsafe { WindowHandle::borrow_raw(self.shared.handles.window) })
    }
}

impl HasDisplayHandle for Window {
    fn display_handle(&self) -> Result<DisplayHandle<'_>, HandleError> {
        // SAFETY: the display outlives every handle borrowed from it.
        Ok(unsafe { DisplayHandle::borrow_raw(self.shared.handles.display) })
    }
}

impl Drop for Window {
    fn drop(&mut self) {
        self.request_close();
    }
}
