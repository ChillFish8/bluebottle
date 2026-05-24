mod state;

use std::ffi::c_void;
use std::ptr::NonNull;
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex, mpsc};
use std::thread;
use std::time::Duration;

use iced::Program;
use smithay_client_toolkit::compositor::CompositorState;
use smithay_client_toolkit::output::OutputState;
use smithay_client_toolkit::reexports::calloop::EventLoop;
use smithay_client_toolkit::reexports::calloop_wayland_source::WaylandSource;
use smithay_client_toolkit::reexports::client::globals::registry_queue_init;
use smithay_client_toolkit::reexports::client::{Connection, Proxy};
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
    PoolSnafu,
    RegistrySnafu,
    SpawnThreadSnafu,
};
use crate::handle::{RawHandles, Shared, Window};
use crate::overlay::IcedOverlay;

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
    F: FnOnce() -> P + 'static,
    P: Program + 'static,
{
    let (conn, mut event_loop, mut state) = match setup(build, tx.clone()) {
        Ok(parts) => parts,
        Err(err) => {
            let _ = tx.send(Err(err));
            return Ok(());
        },
    };

    let _ = conn;

    // Readiness has already been reported, so any failure here surfaces only
    // through the thread's return value (observable via `Window::join`).
    while !state.should_exit() {
        event_loop
            .dispatch(DISPATCH_TIMEOUT, &mut state)
            .context(EventLoopSnafu)?;
        state.tick();
    }

    Ok(())
}

/// Connect to the compositor and build all Wayland state.
#[allow(clippy::type_complexity)]
fn setup<P, F>(
    build: F,
    tx: mpsc::Sender<InitResult>,
) -> Result<(Connection, EventLoop<'static, State>, State), Error>
where
    F: FnOnce() -> P + 'static,
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

    let (width, height) = DEFAULT_SIZE;
    let pool_len = (width * height * 4 * 2) as usize;
    let pool = SlotPool::new(pool_len, &shm).context(PoolSnafu)?;

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
    let handles = RawHandles {
        display: display_ptr,
        surface: NonNull::new(main_surface.id().as_ptr() as *mut c_void)
            .expect("wl_surface pointer is never null"),
    };

    // Build the Iced program and its renderer on this thread (the high-level
    // builder is not Send), targeting the overlay subsurface.
    let overlay_surface_ptr = NonNull::new(overlay_surface.id().as_ptr() as *mut c_void)
        .expect("wl_surface pointer is never null");
    let overlay = Box::new(IcedOverlay::new(
        build(),
        display_ptr,
        overlay_surface_ptr,
        width,
        height,
        1.0,
    )?);

    let shared = Arc::new(Shared {
        handles,
        size: Mutex::new((width, height)),
        scale: Mutex::new(1.0),
        close_requested: AtomicBool::new(false),
    });

    let state = State {
        registry_state: RegistryState::new(&globals),
        output_state: OutputState::new(&globals, &qh),
        seat_state: SeatState::new(&globals, &qh),
        shm,
        pool,
        window,
        main_surface,
        overlay_surface,
        overlay_subsurface,
        overlay,
        width,
        height,
        scale: 1,
        configured: false,
        exit: false,
        needs_redraw: false,
        pointer: None,
        keyboard: None,
        modifiers: iced::keyboard::Modifiers::empty(),
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

    Ok((conn, event_loop, state))
}
