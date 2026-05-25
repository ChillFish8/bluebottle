mod input;
mod state;
mod text_input;
mod viewport;

use std::ffi::c_void;
use std::ptr::NonNull;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Condvar, Mutex, mpsc};
use std::thread;
use std::time::Duration;

use iced::Program;
use iced_runtime::core::input_method::InputMethod;
use raw_window_handle::{
    DisplayHandle,
    HandleError,
    HasDisplayHandle,
    HasWindowHandle,
    RawDisplayHandle,
    RawWindowHandle,
    WaylandDisplayHandle,
    WaylandWindowHandle,
    WindowHandle,
};
use smithay_client_toolkit::compositor::CompositorState;
use smithay_client_toolkit::globals::GlobalData;
use smithay_client_toolkit::output::OutputState;
use smithay_client_toolkit::reexports::calloop::EventLoop;
use smithay_client_toolkit::reexports::calloop::ping::make_ping;
use smithay_client_toolkit::reexports::calloop_wayland_source::WaylandSource;
use smithay_client_toolkit::reexports::client::globals::registry_queue_init;
use smithay_client_toolkit::reexports::client::protocol::wl_shm;
use smithay_client_toolkit::reexports::client::{Connection, Proxy};
use smithay_client_toolkit::reexports::protocols::wp::text_input::zv3::client::zwp_text_input_manager_v3::ZwpTextInputManagerV3;
use smithay_client_toolkit::reexports::protocols::wp::viewporter::client::wp_viewporter::WpViewporter;
use smithay_client_toolkit::registry::RegistryState;
use smithay_client_toolkit::seat::SeatState;
use smithay_client_toolkit::shell::WaylandSurface;
use smithay_client_toolkit::shell::xdg::XdgShell;
use smithay_client_toolkit::shell::xdg::window::WindowDecorations;
use smithay_client_toolkit::shm::Shm;
use smithay_client_toolkit::shm::slot::SlotPool;
use smithay_client_toolkit::subcompositor::SubcompositorState;
use snafu::ResultExt;
use state::State;

use crate::error::{
    ConnectSnafu,
    Error,
    EventLoopSnafu,
    MissingGlobalSnafu,
    RegistrySnafu,
    SpawnThreadSnafu,
};
use crate::handle::{RawHandles, Shared, Window};
use crate::overlay;

/// Wayland-specific extensions to [`Window`].
///
/// Mirrors `winit::platform::wayland::WindowExtWayland`: it exposes the raw
/// `wl_display`/`wl_surface` pointers of the main (parent) surface for APIs that
/// want them directly (for example libmpv's render API).
pub trait WindowExtWayland {
    /// Returns a pointer to the main surface's `wl_display`.
    fn wl_display_ptr(&self) -> *mut c_void;

    /// Returns a pointer to the main `wl_surface`.
    fn wl_surface_ptr(&self) -> *mut c_void;

    /// Returns a pointer to the content `wl_surface` a video sink should render
    /// into, or null when the window was not created with
    /// [`crate::create_video_overlay`].
    ///
    /// The library stacks this surface beneath the overlay, so video drawn here
    /// (for example by GStreamer's `waylandsink`) composites below the UI.
    fn wl_video_surface_ptr(&self) -> *mut c_void;
}

impl WindowExtWayland for Window {
    fn wl_display_ptr(&self) -> *mut c_void {
        match self.raw_display_handle() {
            RawDisplayHandle::Wayland(handle) => handle.display.as_ptr(),
            _ => std::ptr::null_mut(),
        }
    }

    fn wl_surface_ptr(&self) -> *mut c_void {
        match self.raw_window_handle() {
            RawWindowHandle::Wayland(handle) => handle.surface.as_ptr(),
            _ => std::ptr::null_mut(),
        }
    }

    fn wl_video_surface_ptr(&self) -> *mut c_void {
        self.raw_video_surface()
            .map_or(std::ptr::null_mut(), NonNull::as_ptr)
    }
}

/// Raw Wayland handles for the overlay surface, in `raw-window-handle` form.
///
/// Wrapped so the (otherwise `!Send`) pointers can move to the render thread and
/// satisfy the renderer's `HasWindowHandle`/`HasDisplayHandle` requirement.
#[derive(Clone, Copy)]
struct RawSurface {
    display: NonNull<c_void>,
    surface: NonNull<c_void>,
}

// SAFETY: the pointers reference long-lived `wl_display`/`wl_surface` objects;
// libwayland access is internally synchronised.
unsafe impl Send for RawSurface {}
unsafe impl Sync for RawSurface {}

impl HasDisplayHandle for RawSurface {
    fn display_handle(&self) -> Result<DisplayHandle<'_>, HandleError> {
        let raw = RawDisplayHandle::Wayland(WaylandDisplayHandle::new(self.display));
        // SAFETY: the display outlives every handle borrowed from it.
        Ok(unsafe { DisplayHandle::borrow_raw(raw) })
    }
}

impl HasWindowHandle for RawSurface {
    fn window_handle(&self) -> Result<WindowHandle<'_>, HandleError> {
        let raw = RawWindowHandle::Wayland(WaylandWindowHandle::new(self.surface));
        // SAFETY: the surface outlives every handle borrowed from it.
        Ok(unsafe { WindowHandle::borrow_raw(raw) })
    }
}

/// The readiness message sent from the loop thread once the main surface exists.
type InitResult = Result<Arc<Shared>, Error>;

/// The size used until the compositor suggests one of its own.
const DEFAULT_SIZE: (u32, u32) = (1280, 720);

/// Spawn the Wayland event loop on a background thread and return a [`Window`].
///
/// Blocks only until the loop reports that the main surface is ready (or fails);
/// the loop then keeps running on its thread until close is requested.
pub(crate) fn run<P, F>(build: F, video: bool) -> Result<Window, Error>
where
    F: FnOnce() -> P + Send + 'static,
    P: Program + 'static,
{
    let (tx, rx) = mpsc::channel::<InitResult>();

    let thread = thread::Builder::new()
        .name("bluebottle-window".to_owned())
        .spawn(move || event_loop(build, tx, video))
        .context(SpawnThreadSnafu)?;

    match rx.recv() {
        Ok(Ok(shared)) => Ok(Window::new(shared, thread)),
        Ok(Err(err)) => {
            let _ = thread.join();
            Err(err)
        },
        Err(_) => {
            let _ = thread.join();
            Err(Error::LoopExited)
        },
    }
}

/// The body of the background thread: drive Wayland and render the overlay.
fn event_loop<P, F>(
    build: F,
    tx: mpsc::Sender<InitResult>,
    video: bool,
) -> Result<(), Error>
where
    F: FnOnce() -> P + Send + 'static,
    P: Program + 'static,
{
    let (conn, mut event_loop, mut state, render_thread) =
        match setup(build, tx.clone(), video) {
            Ok(parts) => parts,
            Err(err) => {
                let _ = tx.send(Err(err));
                return Ok(());
            },
        };

    // Rendering happens on the overlay thread; here we only dispatch Wayland
    // events (input, configure) and forward them. Keeping this loop dispatching
    // is what lets the caller map the main surface.
    //
    // The dispatch blocks indefinitely: there is no periodic work, so the loop
    // sleeps until either a Wayland event arrives or the render thread / caller
    // signals the wake ping (registered in `setup`). Every wake then drains the
    // render thread's requests and reconciles the pointer/IME, regardless of
    // what caused it — a Wayland event (e.g. pointer enter) needs the same
    // reconciliation as a ping.
    //
    // A dispatch error must still fall through to the shutdown below rather than
    // returning early: the render thread holds a wgpu context referencing the
    // `wl_display`, so the connection must not be dropped until that thread has
    // joined. We capture the error and report it after tearing down cleanly.
    let mut result = Ok(());
    while !state.should_exit() {
        if let Err(err) = event_loop.dispatch(None::<Duration>, &mut state) {
            result = Err(err).context(EventLoopSnafu);
            break;
        }
        // Apply a pending window resize to the video surface, then anything the
        // render thread asked of the toplevel/pointer/IME. Done here, after
        // dispatch returns, rather than inside the `configure` callback — see
        // `apply_pending_resize`.
        state.apply_pending_resize();
        state.apply_window_requests();
        state.sync_cursor(&conn);
        state.sync_ime();
    }

    // Make the shutdown observable to the render thread and the caller, whether
    // the exit came from `request_close`, a compositor close request, or a
    // dispatch error.
    state.shared.close_requested.store(true, Ordering::Release);

    // Join the render thread before the connection drops: its wgpu resources
    // reference the `wl_display`, which is disconnected once `conn` is dropped.
    let _ = render_thread.join();

    // Wait until the caller permits teardown (via `Window::join`/drop), which it
    // does only after stopping anything that references the `wl_display` — e.g. a
    // video sink. Disconnecting the display while such a sink is still presenting
    // deadlocks libwayland. On a compositor-initiated close this parks here until
    // the caller has torn down; on a caller-initiated close it returns at once.
    state.shared.wait_for_teardown();
    drop(conn);

    result
}

/// Connect to the compositor and build all Wayland state.
#[allow(clippy::type_complexity)]
fn setup<P, F>(
    build: F,
    tx: mpsc::Sender<InitResult>,
    video: bool,
) -> Result<
    (
        Connection,
        EventLoop<'static, State>,
        State,
        thread::JoinHandle<()>,
    ),
    Error,
>
where
    F: FnOnce() -> P + Send + 'static,
    P: Program + 'static,
{
    let conn = Connection::connect_to_env().context(ConnectSnafu)?;
    let (globals, event_queue) =
        registry_queue_init::<State>(&conn).context(RegistrySnafu)?;
    let qh = event_queue.handle();

    let compositor =
        CompositorState::bind(&globals, &qh).context(MissingGlobalSnafu {
            what: "wl_compositor",
        })?;
    let subcompositor =
        SubcompositorState::bind(compositor.wl_compositor().clone(), &globals, &qh)
            .context(MissingGlobalSnafu {
                what: "wl_subcompositor",
            })?;
    let xdg_shell = XdgShell::bind(&globals, &qh).context(MissingGlobalSnafu {
        what: "xdg_wm_base",
    })?;
    let shm = Shm::bind(&globals, &qh).context(MissingGlobalSnafu { what: "wl_shm" })?;

    // Text input (IME) is optional: absent on compositors without the protocol.
    let text_input_manager = globals
        .bind::<ZwpTextInputManagerV3, _, _>(&qh, 1..=1, GlobalData)
        .ok();

    // Viewporter scales the 1×1 video-mode backdrop to the window (see
    // `viewport`); optional, though near-universally supported.
    let viewporter = globals
        .bind::<WpViewporter, _, _>(&qh, 1..=1, GlobalData)
        .ok();

    // A dedicated surface the themed pointer presents cursor images on.
    let cursor_surface = compositor.create_surface(&qh);

    let (width, height) = DEFAULT_SIZE;

    // The main surface is the xdg toplevel surface, the subsurface parent, and
    // the surface handed back to the caller to render into.
    let main_surface = compositor.create_surface(&qh);
    let window = xdg_shell.create_window(
        main_surface.clone(),
        WindowDecorations::RequestServer,
        &qh,
    );
    window.set_title("Bluebottle");
    window.set_app_id("dev.bluebottle.window");
    window.set_min_size(Some((1, 1)));
    window.commit();

    // In video mode, create a transparent content subsurface *below* the overlay
    // for an external video sink to render into. Creating it before the overlay
    // subsurface leaves the overlay on top (a new subsurface is stacked
    // top-most), so the sink's video composites beneath the UI.
    let (content_surface, content_subsurface, video_pool, content_buffer, video_ptr) =
        if video {
            let (content_subsurface, content_surface) =
                subcompositor.create_subsurface(main_surface.clone(), &qh);
            content_subsurface.set_position(0, 0);
            content_subsurface.set_desync();

            // The parent must have a committed buffer to map, but the sink's own
            // (larger) subsurface provides the visible video — a 1x1 transparent
            // placeholder is enough. Commit it once; the sink owns it thereafter.
            let mut pool =
                SlotPool::new(16, &shm).map_err(|err| Error::VideoBuffer {
                    message: err.to_string(),
                })?;
            let buffer = {
                let (buffer, canvas) = pool
                    .create_buffer(1, 1, 4, wl_shm::Format::Argb8888)
                    .map_err(|err| Error::VideoBuffer {
                        message: err.to_string(),
                    })?;
                canvas.fill(0);
                buffer
            };
            buffer
                .attach_to(&content_surface)
                .map_err(|err| Error::VideoBuffer {
                    message: err.to_string(),
                })?;
            content_surface.damage_buffer(0, 0, 1, 1);
            content_surface.commit();

            let ptr = NonNull::new(content_surface.id().as_ptr() as *mut c_void)
                .expect("wl_surface pointer is never null");
            (
                Some(content_surface),
                Some(content_subsurface),
                Some(pool),
                Some(buffer),
                Some(ptr),
            )
        } else {
            (None, None, None, None, None)
        };

    // In video mode, scale the main surface's 1×1 backdrop to the window with a
    // viewport so resizes need no shm reallocation.
    let viewport = if video {
        viewporter
            .as_ref()
            .map(|vp| vp.get_viewport(&main_surface, &qh, GlobalData))
    } else {
        None
    };

    let (overlay_subsurface, overlay_surface) =
        subcompositor.create_subsurface(main_surface.clone(), &qh);
    overlay_subsurface.set_position(0, 0);
    overlay_subsurface.set_desync();

    let display_ptr = NonNull::new(conn.backend().display_ptr() as *mut c_void)
        .expect("wl_display pointer is never null");
    let main_surface_ptr = NonNull::new(main_surface.id().as_ptr() as *mut c_void)
        .expect("wl_surface pointer is never null");
    let handles = RawHandles {
        window: RawWindowHandle::Wayland(WaylandWindowHandle::new(main_surface_ptr)),
        display: RawDisplayHandle::Wayland(WaylandDisplayHandle::new(display_ptr)),
        video: video_ptr,
    };

    let overlay_surface_ptr = NonNull::new(overlay_surface.id().as_ptr() as *mut c_void)
        .expect("wl_surface pointer is never null");

    // Both loops block when idle, so each needs rousing: the dispatch loop by a
    // calloop ping (its source is registered with the event loop below), and the
    // render loop by a `Tick::Wake` on its command channel. `Shared::wake`
    // signals both, so a caller close request or a render-thread update reaches
    // whichever loop must act. The `Ping` shares its eventfd with the source, so
    // it stays valid for as long as `Shared` lives.
    let (wake_ping, wake_source) =
        make_ping().map_err(|err| Error::EventLoopInsert {
            message: format!("failed to create the wake ping: {err}"),
        })?;

    // Wayland-thread commands and bare wakeups share one channel so the render
    // loop can block on it. The Wayland thread keeps `commands_tx` (in `State`);
    // the wake closure and the runtime hold further senders.
    let (commands_tx, commands_rx) = mpsc::channel::<overlay::Tick>();

    let shared = Arc::new(Shared {
        handles,
        size: Mutex::new((width, height)),
        scale: Mutex::new(1.0),
        close_requested: AtomicBool::new(false),
        cursor: Mutex::new(Default::default()),
        ime: Mutex::new(InputMethod::Disabled),
        resize: Mutex::new(None),
        wake: Arc::new({
            let wake_tick = commands_tx.clone();
            move || {
                wake_ping.ping();
                let _ = wake_tick.send(overlay::Tick::Wake);
            }
        }),
        teardown_permitted: (Mutex::new(false), Condvar::new()),
    });

    // Spawn the render thread. It builds the Iced overlay (neither the program
    // nor the wgpu renderer is Send) and renders there, so blocking on surface
    // presentation never stalls this event loop. We wait for it to finish
    // building before continuing, so renderer errors are reported eagerly.
    let (window_tx, window_rx) = mpsc::channel::<overlay::WindowRequest>();
    let (ready_tx, ready_rx) = mpsc::channel::<Result<(), Error>>();
    let overlay_target = RawSurface {
        display: display_ptr,
        surface: overlay_surface_ptr,
    };
    let render_shared = Arc::clone(&shared);
    // The render loop uses this to let async runtime output wake itself.
    let render_notify = commands_tx.clone();
    let render_thread = thread::Builder::new()
        .name("bluebottle-overlay".to_owned())
        .spawn(move || {
            // Rebind so the closure captures the (`Send`) `RawSurface` as a unit
            // rather than its individual `!Send` pointer fields (Rust 2021 precise capture).
            let overlay_target = overlay_target;
            overlay::run(
                build,
                overlay_target,
                (width, height),
                1.0,
                commands_rx,
                render_notify,
                window_tx,
                render_shared,
                ready_tx,
            );
        })
        .context(SpawnThreadSnafu)?;

    match ready_rx.recv() {
        Ok(Ok(())) => {},
        Ok(Err(err)) => return Err(err),
        Err(_) => return Err(Error::LoopExited),
    }

    let state = State {
        registry_state: RegistryState::new(&globals),
        output_state: OutputState::new(&globals, &qh),
        seat_state: SeatState::new(&globals, &qh),
        window,
        main_surface,
        overlay_surface,
        overlay_subsurface,
        content_surface,
        content_subsurface,
        video_pool,
        main_buffer: None,
        content_buffer,
        commands: commands_tx,
        window_requests: window_rx,
        shm,
        cursor_surface,
        width,
        height,
        scale: 1,
        configured: false,
        exit: false,
        resizing: false,
        focused: false,
        maximized: false,
        fullscreen: false,
        decorated: true,
        current_output: None,
        themed_pointer: None,
        keyboard: None,
        modifiers: iced::keyboard::Modifiers::empty(),
        text_input_manager,
        text_input: None,
        ime_entered: false,
        ime_enabled: false,
        ime_serial: 0,
        ime_preedit: None,
        ime_commit: None,
        ime_applied: InputMethod::Disabled,
        pointer_on_surface: false,
        applied_cursor: None,
        pending_video_resize: None,
        viewport,
        shared,
        init_tx: Some(tx),
    };

    let event_loop: EventLoop<'static, State> =
        EventLoop::try_new().context(EventLoopSnafu)?;
    WaylandSource::new(conn.clone(), event_queue)
        .insert(event_loop.handle())
        .map_err(|err| Error::EventLoopInsert {
            message: err.to_string(),
        })?;

    // The ping only needs to wake the blocked dispatch; the reconciliation it
    // stands in for runs unconditionally after each dispatch turn, so the
    // callback itself is empty.
    event_loop
        .handle()
        .insert_source(wake_source, |_event, _meta, _state| {})
        .map_err(|err| Error::EventLoopInsert {
            message: format!("failed to register the wake source: {err}"),
        })?;

    Ok((conn, event_loop, state, render_thread))
}
