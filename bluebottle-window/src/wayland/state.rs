use std::sync::atomic::Ordering;
use std::sync::{Arc, mpsc};

use iced_runtime::core::{Event, Point, keyboard, mouse};
use smithay_client_toolkit::compositor::CompositorHandler;
use smithay_client_toolkit::output::{OutputHandler, OutputState};
use smithay_client_toolkit::reexports::client::protocol::{
    wl_keyboard,
    wl_output,
    wl_pointer,
    wl_seat,
    wl_shm,
    wl_subsurface,
    wl_surface,
};
use smithay_client_toolkit::reexports::client::{Connection, QueueHandle};
use smithay_client_toolkit::registry::{ProvidesRegistryState, RegistryState};
use smithay_client_toolkit::seat::keyboard::{
    KeyEvent,
    KeyboardHandler,
    Modifiers as SctkModifiers,
    RawModifiers,
};
use smithay_client_toolkit::seat::pointer::{
    PointerEvent,
    PointerEventKind,
    PointerHandler,
};
use smithay_client_toolkit::seat::{Capability, SeatHandler, SeatState};
use smithay_client_toolkit::shell::xdg::window::{
    Window,
    WindowConfigure,
    WindowHandler,
};
use smithay_client_toolkit::shm::slot::SlotPool;
use smithay_client_toolkit::shm::{Shm, ShmHandler};
use smithay_client_toolkit::{
    delegate_compositor,
    delegate_keyboard,
    delegate_output,
    delegate_pointer,
    delegate_registry,
    delegate_seat,
    delegate_shm,
    delegate_subcompositor,
    delegate_xdg_shell,
    delegate_xdg_window,
    registry_handlers,
};
use snafu::ResultExt;

use crate::error::{BufferSnafu, Error};
use crate::handle::Shared;
use crate::overlay::{Overlay, input};

/// The opaque fill used for the main surface as a stand-in for caller content.
///
/// Stored as little-endian `Argb8888`, i.e. `[B, G, R, A]`. Real callers draw
/// the main surface themselves (e.g. video); this only proves compositing.
const MAIN_FILL: [u8; 4] = [0x20, 0x20, 0x20, 0xFF];

/// All Wayland state owned by the event loop thread.
pub(crate) struct State {
    pub registry_state: RegistryState,
    pub output_state: OutputState,
    pub seat_state: SeatState,
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
    pub needs_redraw: bool,

    pub pointer: Option<wl_pointer::WlPointer>,
    pub keyboard: Option<wl_keyboard::WlKeyboard>,
    pub modifiers: keyboard::Modifiers,

    pub shared: Arc<Shared>,
    pub init_tx: Option<mpsc::Sender<Result<Arc<Shared>, Error>>>,
}

impl State {
    /// The current size of the surfaces in physical pixels.
    fn physical_size(&self) -> (u32, u32) {
        let scale = self.scale.max(1) as u32;
        ((self.width * scale).max(1), (self.height * scale).max(1))
    }

    /// Apply the current logical size and scale to both surfaces.
    ///
    /// Sets the buffer scale so physical-pixel buffers map to the logical
    /// window size, and resizes the overlay's wgpu surface and viewport.
    fn apply_layout(&mut self) {
        let scale = self.scale.max(1);
        self.main_surface.set_buffer_scale(scale);
        self.overlay_surface.set_buffer_scale(scale);
        self.overlay.resize(self.width, self.height, scale as f64);
    }

    /// Render the Iced overlay and refresh the main-surface stand-in.
    ///
    /// The overlay presents its own (transparent) surface via wgpu; committing
    /// the parent afterwards latches the subsurface into place.
    pub fn draw(&mut self) -> Result<(), Error> {
        let (width, height) = self.physical_size();
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
            self.apply_layout();
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

        *self.shared.size.lock().expect("size mutex poisoned") =
            (self.width, self.height);

        self.apply_layout();
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

    registry_handlers![OutputState, SeatState];
}

impl State {
    /// Whether the event loop should stop on the next turn.
    pub fn should_exit(&self) -> bool {
        self.exit || self.shared.close_requested.load(Ordering::Acquire)
    }

    /// Advance the overlay once per event-loop turn.
    ///
    /// Drains async runtime output, honours a program-requested exit, and
    /// redraws the overlay if input, async messages, or a pending animation
    /// frame requires it. The main surface is owned by the caller and is left
    /// untouched here.
    pub fn tick(&mut self) {
        let async_dirty = self.overlay.pump();

        if self.overlay.should_exit() {
            self.exit = true;
            return;
        }

        let had_input = std::mem::take(&mut self.needs_redraw);
        if had_input || async_dirty || self.overlay.wants_redraw() {
            self.overlay.draw();
        }
    }
}

impl SeatHandler for State {
    fn seat_state(&mut self) -> &mut SeatState {
        &mut self.seat_state
    }

    fn new_seat(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _seat: wl_seat::WlSeat,
    ) {
    }

    fn new_capability(
        &mut self,
        _conn: &Connection,
        qh: &QueueHandle<Self>,
        seat: wl_seat::WlSeat,
        capability: Capability,
    ) {
        match capability {
            Capability::Pointer if self.pointer.is_none() => {
                if let Ok(pointer) = self.seat_state.get_pointer(qh, &seat) {
                    self.pointer = Some(pointer);
                }
            },
            Capability::Keyboard if self.keyboard.is_none() => {
                match self
                    .seat_state
                    .get_keyboard::<State, State>(qh, &seat, None)
                {
                    Ok(keyboard) => self.keyboard = Some(keyboard),
                    Err(err) => tracing::warn!("failed to get keyboard: {err}"),
                }
            },
            _ => {},
        }
    }

    fn remove_capability(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _seat: wl_seat::WlSeat,
        capability: Capability,
    ) {
        match capability {
            Capability::Pointer => {
                if let Some(pointer) = self.pointer.take() {
                    pointer.release();
                }
            },
            Capability::Keyboard => {
                if let Some(keyboard) = self.keyboard.take() {
                    keyboard.release();
                }
            },
            _ => {},
        }
    }

    fn remove_seat(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _seat: wl_seat::WlSeat,
    ) {
    }
}

impl PointerHandler for State {
    fn pointer_frame(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _pointer: &wl_pointer::WlPointer,
        events: &[PointerEvent],
    ) {
        for event in events {
            let position = Point::new(event.position.0 as f32, event.position.1 as f32);

            match &event.kind {
                PointerEventKind::Enter { .. } => {
                    self.overlay.set_cursor(Some(position));
                    self.overlay
                        .queue_event(Event::Mouse(mouse::Event::CursorEntered));
                    self.overlay
                        .queue_event(Event::Mouse(mouse::Event::CursorMoved {
                            position,
                        }));
                },
                PointerEventKind::Leave { .. } => {
                    self.overlay.set_cursor(None);
                    self.overlay
                        .queue_event(Event::Mouse(mouse::Event::CursorLeft));
                },
                PointerEventKind::Motion { .. } => {
                    self.overlay.set_cursor(Some(position));
                    self.overlay
                        .queue_event(Event::Mouse(mouse::Event::CursorMoved {
                            position,
                        }));
                },
                PointerEventKind::Press { button, .. } => {
                    self.overlay
                        .queue_event(Event::Mouse(mouse::Event::ButtonPressed(
                            input::mouse_button(*button),
                        )));
                },
                PointerEventKind::Release { button, .. } => {
                    self.overlay.queue_event(Event::Mouse(
                        mouse::Event::ButtonReleased(input::mouse_button(*button)),
                    ));
                },
                PointerEventKind::Axis {
                    horizontal,
                    vertical,
                    ..
                } => {
                    let delta = mouse::ScrollDelta::Pixels {
                        x: -horizontal.absolute as f32,
                        y: -vertical.absolute as f32,
                    };
                    self.overlay.queue_event(Event::Mouse(
                        mouse::Event::WheelScrolled { delta },
                    ));
                },
            }

            self.needs_redraw = true;
        }
    }
}

impl KeyboardHandler for State {
    fn enter(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _keyboard: &wl_keyboard::WlKeyboard,
        _surface: &wl_surface::WlSurface,
        _serial: u32,
        _raw: &[u32],
        _keysyms: &[smithay_client_toolkit::seat::keyboard::Keysym],
    ) {
    }

    fn leave(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _keyboard: &wl_keyboard::WlKeyboard,
        _surface: &wl_surface::WlSurface,
        _serial: u32,
    ) {
    }

    fn press_key(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _keyboard: &wl_keyboard::WlKeyboard,
        _serial: u32,
        event: KeyEvent,
    ) {
        self.overlay.queue_event(Event::Keyboard(input::key_event(
            event.keysym,
            event.utf8,
            self.modifiers,
            true,
            false,
        )));
        self.needs_redraw = true;
    }

    fn repeat_key(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _keyboard: &wl_keyboard::WlKeyboard,
        _serial: u32,
        event: KeyEvent,
    ) {
        self.overlay.queue_event(Event::Keyboard(input::key_event(
            event.keysym,
            event.utf8,
            self.modifiers,
            true,
            true,
        )));
        self.needs_redraw = true;
    }

    fn release_key(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _keyboard: &wl_keyboard::WlKeyboard,
        _serial: u32,
        event: KeyEvent,
    ) {
        self.overlay.queue_event(Event::Keyboard(input::key_event(
            event.keysym,
            event.utf8,
            self.modifiers,
            false,
            false,
        )));
        self.needs_redraw = true;
    }

    fn update_modifiers(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _keyboard: &wl_keyboard::WlKeyboard,
        _serial: u32,
        modifiers: SctkModifiers,
        _raw_modifiers: RawModifiers,
        _layout: u32,
    ) {
        self.modifiers = input::modifiers(modifiers);
        self.overlay
            .queue_event(Event::Keyboard(keyboard::Event::ModifiersChanged(
                self.modifiers,
            )));
        self.needs_redraw = true;
    }
}

delegate_compositor!(State);
delegate_subcompositor!(State);
delegate_output!(State);
delegate_seat!(State);
delegate_pointer!(State);
delegate_keyboard!(State);
delegate_shm!(State);
delegate_xdg_shell!(State);
delegate_xdg_window!(State);
delegate_registry!(State);
