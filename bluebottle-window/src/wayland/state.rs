use std::sync::atomic::Ordering;
use std::sync::{Arc, mpsc};

use smithay_client_toolkit::compositor::CompositorHandler;
use smithay_client_toolkit::output::{OutputHandler, OutputState};
use smithay_client_toolkit::reexports::client::protocol::{
    wl_output,
    wl_shm,
    wl_subsurface,
    wl_surface,
};
use smithay_client_toolkit::reexports::client::{Connection, QueueHandle};
use smithay_client_toolkit::registry::{ProvidesRegistryState, RegistryState};
use smithay_client_toolkit::shell::xdg::window::{
    Window,
    WindowConfigure,
    WindowHandler,
};
use smithay_client_toolkit::shm::slot::SlotPool;
use smithay_client_toolkit::shm::{Shm, ShmHandler};
use smithay_client_toolkit::{
    delegate_compositor,
    delegate_output,
    delegate_registry,
    delegate_shm,
    delegate_subcompositor,
    delegate_xdg_shell,
    delegate_xdg_window,
    registry_handlers,
};
use snafu::ResultExt;

use crate::error::{BufferSnafu, Error};
use crate::handle::Shared;
use crate::overlay::Overlay;

/// The opaque fill used for the main surface as a stand-in for caller content.
///
/// Stored as little-endian `Argb8888`, i.e. `[B, G, R, A]`. Real callers draw
/// the main surface themselves (e.g. video); this only proves compositing.
const MAIN_FILL: [u8; 4] = [0x20, 0x20, 0x20, 0xFF];

/// All Wayland state owned by the event loop thread.
pub(crate) struct State {
    pub registry_state: RegistryState,
    pub output_state: OutputState,
    pub shm: Shm,

    pub pool: SlotPool,
    // Retained to own the underlying Wayland objects; dropping these would
    // destroy the toplevel/subsurface. Read from Phase 6 (resize handling).
    #[allow(dead_code)]
    pub window: Window,
    pub main_surface: wl_surface::WlSurface,
    #[allow(dead_code)]
    pub overlay_surface: wl_surface::WlSurface,
    #[allow(dead_code)]
    pub overlay_subsurface: wl_subsurface::WlSubsurface,
    pub overlay: Box<dyn Overlay>,

    pub width: u32,
    pub height: u32,
    pub scale: i32,

    pub configured: bool,
    pub exit: bool,

    pub shared: Arc<Shared>,
    pub init_tx: Option<mpsc::Sender<Result<Arc<Shared>, Error>>>,
}

impl State {
    /// Render the Iced overlay and refresh the main-surface stand-in.
    ///
    /// The overlay presents its own (transparent) surface via wgpu; committing
    /// the parent afterwards latches the subsurface into place.
    pub fn draw(&mut self) -> Result<(), Error> {
        let (width, height) = (self.width.max(1), self.height.max(1));
        let stride = width as i32 * 4;

        self.overlay.draw();
        paint(
            &mut self.pool,
            &self.main_surface,
            width,
            height,
            stride,
            MAIN_FILL,
        )?;

        Ok(())
    }

    /// Report readiness to [`crate::create_overlay`] exactly once.
    fn announce_ready(&mut self) {
        if let Some(tx) = self.init_tx.take() {
            *self.shared.size.lock().expect("size mutex poisoned") =
                (self.width, self.height);
            *self.shared.scale.lock().expect("scale mutex poisoned") = self.scale as f64;
            let _ = tx.send(Ok(Arc::clone(&self.shared)));
        }
    }
}

/// Fill `surface` with a solid colour from a freshly created shm buffer.
fn paint(
    pool: &mut SlotPool,
    surface: &wl_surface::WlSurface,
    width: u32,
    height: u32,
    stride: i32,
    fill: [u8; 4],
) -> Result<(), Error> {
    let (buffer, canvas) = pool
        .create_buffer(
            width as i32,
            height as i32,
            stride,
            wl_shm::Format::Argb8888,
        )
        .context(BufferSnafu)?;

    for pixel in canvas.chunks_exact_mut(4) {
        pixel.copy_from_slice(&fill);
    }

    buffer.attach_to(surface).expect("attach buffer to surface");
    surface.damage_buffer(0, 0, width as i32, height as i32);
    surface.commit();

    Ok(())
}

impl CompositorHandler for State {
    fn scale_factor_changed(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _surface: &wl_surface::WlSurface,
        new_factor: i32,
    ) {
        self.scale = new_factor;
        *self.shared.scale.lock().expect("scale mutex poisoned") = new_factor as f64;

        if self.configured {
            self.overlay
                .resize(self.width, self.height, new_factor as f64);
            let _ = self.draw();
        }
    }

    fn transform_changed(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _surface: &wl_surface::WlSurface,
        _new_transform: wl_output::Transform,
    ) {
    }

    fn frame(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _surface: &wl_surface::WlSurface,
        _time: u32,
    ) {
        // Static content for now; nothing to advance on each frame callback.
    }

    fn surface_enter(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _surface: &wl_surface::WlSurface,
        _output: &wl_output::WlOutput,
    ) {
    }

    fn surface_leave(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _surface: &wl_surface::WlSurface,
        _output: &wl_output::WlOutput,
    ) {
    }
}

impl WindowHandler for State {
    fn request_close(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _window: &Window,
    ) {
        self.exit = true;
    }

    fn configure(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _window: &Window,
        configure: WindowConfigure,
        _serial: u32,
    ) {
        if let Some(width) = configure.new_size.0 {
            self.width = width.get();
        }
        if let Some(height) = configure.new_size.1 {
            self.height = height.get();
        }

        self.overlay
            .resize(self.width, self.height, self.scale as f64);
        let _ = self.draw();
        self.configured = true;
        self.announce_ready();
    }
}

impl OutputHandler for State {
    fn output_state(&mut self) -> &mut OutputState {
        &mut self.output_state
    }

    fn new_output(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _output: wl_output::WlOutput,
    ) {
    }

    fn update_output(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _output: wl_output::WlOutput,
    ) {
    }

    fn output_destroyed(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _output: wl_output::WlOutput,
    ) {
    }
}

impl ShmHandler for State {
    fn shm_state(&mut self) -> &mut Shm {
        &mut self.shm
    }
}

impl ProvidesRegistryState for State {
    fn registry(&mut self) -> &mut RegistryState {
        &mut self.registry_state
    }

    registry_handlers![OutputState];
}

impl State {
    /// Whether the event loop should stop on the next turn.
    pub fn should_exit(&self) -> bool {
        self.exit || self.shared.close_requested.load(Ordering::Acquire)
    }
}

delegate_compositor!(State);
delegate_subcompositor!(State);
delegate_output!(State);
delegate_shm!(State);
delegate_xdg_shell!(State);
delegate_xdg_window!(State);
delegate_registry!(State);
