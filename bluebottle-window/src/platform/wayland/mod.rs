mod input;
mod state;
mod text_input;

use std::ffi::c_void;
use std::ptr::NonNull;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, mpsc};
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
use smithay_client_toolkit::reexports::calloop_wayland_source::WaylandSource;
use smithay_client_toolkit::reexports::client::globals::registry_queue_init;
use smithay_client_toolkit::reexports::client::{Connection, Proxy};
use smithay_client_toolkit::reexports::protocols::wp::text_input::zv3::client::zwp_text_input_manager_v3::ZwpTextInputManagerV3;
use smithay_client_toolkit::registry::RegistryState;
use smithay_client_toolkit::seat::SeatState;
use smithay_client_toolkit::shell::WaylandSurface;
use smithay_client_toolkit::shell::xdg::XdgShell;
use smithay_client_toolkit::shell::xdg::window::WindowDecorations;
use smithay_client_toolkit::shm::Shm;
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

/// How long the loop blocks per turn before re-checking for a close request.
const DISPATCH_TIMEOUT: Duration = Duration::from_millis(16);

/// Spawn the Wayland event loop on a background thread and return a [`Window`].
///
/// Blocks only until the loop reports that the main surface is ready (or fails);
/// the loop then keeps running on its thread until close is requested.
pub(crate) fn run<P, F>(build: F) -> Result<Window, Error>
where
    F: FnOnce() -> P + Send + 'static,
    P: Program + 'static,
{
    let (tx, rx) = mpsc::channel::<InitResult>();

    let thread = thread::Builder::new()
        .name("bluebottle-window".to_owned())
        .spawn(move || event_loop(build, tx))
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
fn event_loop<P, F>(build: F, tx: mpsc::Sender<InitResult>) -> Result<(), Error>
where
    F: FnOnce() -> P + Send + 'static,
    P: Program + 'static,
{
    let (conn, mut event_loop, mut state, render_thread) = match setup(build, tx.clone())
    {
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
    // A dispatch error must still fall through to the shutdown below rather than
    // returning early: the render thread holds a wgpu context referencing the
    // `wl_display`, so the connection must not be dropped until that thread has
    // joined. We capture the error and report it after tearing down cleanly.
    let mut result = Ok(());
    while !state.should_exit() {
        if let Err(err) = event_loop.dispatch(DISPATCH_TIMEOUT, &mut state) {
            result = Err(err).context(EventLoopSnafu);
            break;
        }
        // Apply anything the render thread asked of the toplevel/pointer/IME.
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
    drop(conn);

    result
}

/// Connect to the compositor and build all Wayland state.
#[allow(clippy::type_complexity)]
fn setup<P, F>(
    build: F,
    tx: mpsc::Sender<InitResult>,
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
    };

    let overlay_surface_ptr = NonNull::new(overlay_surface.id().as_ptr() as *mut c_void)
        .expect("wl_surface pointer is never null");

    let shared = Arc::new(Shared {
        handles,
        size: Mutex::new((width, height)),
        scale: Mutex::new(1.0),
        close_requested: AtomicBool::new(false),
        cursor: Mutex::new(Default::default()),
        ime: Mutex::new(InputMethod::Disabled),
    });

    // Spawn the render thread. It builds the Iced overlay (neither the program
    // nor the wgpu renderer is Send) and renders there, so blocking on surface
    // presentation never stalls this event loop. We wait for it to finish
    // building before continuing, so renderer errors are reported eagerly.
    let (commands_tx, commands_rx) = mpsc::channel::<overlay::Command>();
    let (window_tx, window_rx) = mpsc::channel::<overlay::WindowRequest>();
    let (ready_tx, ready_rx) = mpsc::channel::<Result<(), Error>>();
    let overlay_target = RawSurface {
        display: display_ptr,
        surface: overlay_surface_ptr,
    };
    let render_shared = Arc::clone(&shared);
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

    Ok((conn, event_loop, state, render_thread))
}
