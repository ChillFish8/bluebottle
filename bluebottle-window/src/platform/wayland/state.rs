use std::sync::atomic::Ordering;
use std::sync::{Arc, mpsc};

use iced_runtime::core::{Event, Point, Size, input_method, keyboard, mouse, window};
use smithay_client_toolkit::compositor::CompositorHandler;
use smithay_client_toolkit::output::{OutputHandler, OutputState};
use smithay_client_toolkit::reexports::client::protocol::{
    wl_keyboard,
    wl_output,
    wl_pointer,
    wl_seat,
    wl_subsurface,
    wl_surface,
};
use smithay_client_toolkit::reexports::client::{Connection, Proxy, QueueHandle};
use smithay_client_toolkit::reexports::protocols::wp::text_input::zv3::client::zwp_text_input_manager_v3::ZwpTextInputManagerV3;
use smithay_client_toolkit::reexports::protocols::wp::text_input::zv3::client::zwp_text_input_v3::ZwpTextInputV3;
use smithay_client_toolkit::reexports::protocols::xdg::shell::client::xdg_toplevel::ResizeEdge;
use smithay_client_toolkit::registry::{ProvidesRegistryState, RegistryState};
use smithay_client_toolkit::seat::keyboard::{
    KeyEvent,
    KeyboardHandler,
    Modifiers as SctkModifiers,
    RawModifiers,
};
use smithay_client_toolkit::seat::pointer::{
    PointerData,
    PointerEvent,
    PointerEventKind,
    PointerHandler,
    ThemeSpec,
    ThemedPointer,
};
use smithay_client_toolkit::seat::{Capability, SeatHandler, SeatState};
use smithay_client_toolkit::shell::xdg::window::{
    Window,
    WindowConfigure,
    WindowHandler,
};
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

use super::input;
use crate::error::Error;
use crate::handle::Shared;
use crate::overlay::{Command, WindowRequest};

/// All Wayland state owned by the event loop thread.
///
/// Rendering lives on a separate thread (see [`crate::overlay::run`]); this
/// state forwards input and layout changes to it over [`State::commands`] so
/// the event loop never blocks on surface presentation.
pub(crate) struct State {
    pub registry_state: RegistryState,
    pub output_state: OutputState,
    pub seat_state: SeatState,

    // Retained to own the underlying Wayland objects; dropping these would
    // destroy the toplevel/subsurface.
    #[allow(dead_code)]
    pub window: Window,
    pub main_surface: wl_surface::WlSurface,
    pub overlay_surface: wl_surface::WlSurface,
    #[allow(dead_code)]
    pub overlay_subsurface: wl_subsurface::WlSubsurface,
    pub commands: mpsc::Sender<Command>,
    // Window-control requests from the overlay UI (it is the window's chrome).
    pub window_requests: mpsc::Receiver<WindowRequest>,

    // Shared memory + a dedicated cursor surface back the themed pointer.
    pub shm: Shm,
    pub cursor_surface: wl_surface::WlSurface,

    pub width: u32,
    pub height: u32,
    pub scale: i32,

    pub configured: bool,
    pub exit: bool,
    pub resizing: bool,
    pub focused: bool,
    pub maximized: bool,

    pub themed_pointer: Option<ThemedPointer>,
    pub keyboard: Option<wl_keyboard::WlKeyboard>,
    pub modifiers: keyboard::Modifiers,

    // Text input (IME). The manager is bound once; the text input is created
    // per seat. Preedit/commit are accumulated and applied on `done`.
    pub text_input_manager: Option<ZwpTextInputManagerV3>,
    pub text_input: Option<ZwpTextInputV3>,
    pub ime_entered: bool,
    pub ime_enabled: bool,
    pub ime_serial: u32,
    pub ime_preedit: Option<(String, i32, i32)>,
    pub ime_commit: Option<String>,
    pub ime_applied: input_method::InputMethod,

    // Cursor management: whether the pointer is over our surface, and the last
    // interaction we applied (so we only call `set_cursor` on changes).
    pub pointer_on_surface: bool,
    pub applied_cursor: Option<mouse::Interaction>,

    pub shared: Arc<Shared>,
    pub init_tx: Option<mpsc::Sender<Result<Arc<Shared>, Error>>>,
}

impl State {
    /// Send a [`Command`] to the render thread, ignoring a closed channel.
    fn send(&self, command: Command) {
        let _ = self.commands.send(command);
    }

    /// Feed a window lifecycle event into the overlay UI.
    fn send_window_event(&self, event: window::Event) {
        self.send(Command::Event(Event::Window(event)));
    }

    /// Apply the current logical size and scale to both surfaces.
    ///
    /// Sets the buffer scale on each surface so the physical-pixel buffers the
    /// caller and the renderer produce (`logical * scale`) map to the logical
    /// window size, and tells the render thread to resize the overlay.
    fn apply_layout(&mut self) {
        let scale = self.scale.max(1);
        self.main_surface.set_buffer_scale(scale);
        self.overlay_surface.set_buffer_scale(scale);
        self.send(Command::Resize {
            width: self.width,
            height: self.height,
            scale: scale as f64,
        });
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

    /// Apply the cursor the render thread requested, if it changed.
    ///
    /// Called once per event-loop turn. The render thread publishes the desired
    /// [`mouse::Interaction`] to [`Shared`]; here (where the pointer lives) it is
    /// mapped to a cursor and applied, but only while the pointer is over our
    /// surface and only when it differs from what was last set.
    pub fn sync_cursor(&mut self, conn: &Connection) {
        if !self.pointer_on_surface {
            return;
        }
        let Some(pointer) = self.themed_pointer.as_ref() else {
            return;
        };

        let desired = *self.shared.cursor.lock().expect("cursor mutex poisoned");
        if self.applied_cursor == Some(desired) {
            return;
        }

        let result = match input::cursor_icon(desired) {
            Some(icon) => pointer.set_cursor(conn, icon),
            None => pointer.hide_cursor(),
        };
        if result.is_ok() {
            self.applied_cursor = Some(desired);
        }
    }

    /// Apply all window-control requests the overlay UI has queued.
    pub fn apply_window_requests(&mut self) {
        while let Ok(request) = self.window_requests.try_recv() {
            self.apply_window_request(request);
        }
    }

    fn apply_window_request(&mut self, request: WindowRequest) {
        match request {
            WindowRequest::Minimize => self.window.set_minimized(),
            WindowRequest::SetMaximized(true) => self.window.set_maximized(),
            WindowRequest::SetMaximized(false) => self.window.unset_maximized(),
            WindowRequest::ToggleMaximized => {
                if self.maximized {
                    self.window.unset_maximized();
                } else {
                    self.window.set_maximized();
                }
            },
            WindowRequest::SetFullscreen(true) => self.window.set_fullscreen(None),
            WindowRequest::SetFullscreen(false) => self.window.unset_fullscreen(),
            WindowRequest::Drag => {
                if let Some((seat, serial)) = self.pointer_grab() {
                    self.window.move_(&seat, serial);
                }
            },
            WindowRequest::DragResize(direction) => {
                if let Some((seat, serial)) = self.pointer_grab() {
                    self.window.resize(&seat, serial, resize_edge(direction));
                }
            },
        }
    }

    /// The seat and latest button serial for an interactive move/resize grab.
    ///
    /// The compositor requires the serial of the button press that started the
    /// drag; without a pointer (or a press), no grab can be initiated.
    fn pointer_grab(&self) -> Option<(wl_seat::WlSeat, u32)> {
        let pointer = self.themed_pointer.as_ref()?;
        let data = pointer.pointer().data::<PointerData>()?;
        Some((data.seat().clone(), data.latest_button_serial()?))
    }
}

/// Map an Iced resize [`window::Direction`] to an xdg-toplevel [`ResizeEdge`].
fn resize_edge(direction: window::Direction) -> ResizeEdge {
    use window::Direction;

    match direction {
        Direction::North => ResizeEdge::Top,
        Direction::South => ResizeEdge::Bottom,
        Direction::East => ResizeEdge::Right,
        Direction::West => ResizeEdge::Left,
        Direction::NorthEast => ResizeEdge::TopRight,
        Direction::NorthWest => ResizeEdge::TopLeft,
        Direction::SouthEast => ResizeEdge::BottomRight,
        Direction::SouthWest => ResizeEdge::BottomLeft,
    }
}

impl CompositorHandler for State {
    fn scale_factor_changed(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        surface: &wl_surface::WlSurface,
        new_factor: i32,
    ) {
        // Other surfaces (e.g. the themed pointer's cursor surface) also report
        // scale; only the window's scale should drive the overlay.
        if surface != &self.main_surface || new_factor == self.scale {
            return;
        }

        self.scale = new_factor;
        *self.shared.scale.lock().expect("scale mutex poisoned") = new_factor as f64;
        self.send_window_event(window::Event::Rescaled(new_factor as f32));

        if self.configured {
            self.apply_layout();
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
        // Let the UI observe the request, then close. (iced's default is to
        // close on request; intercepting it would need an opt-out flag.)
        self.send_window_event(window::Event::CloseRequested);
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
        // The overlay normally runs desync so its UI can repaint independently
        // of the caller's frames. During an interactive resize, switch it to
        // sync so its new size is applied atomically with the parent's commit
        // rather than racing ahead of (or behind) it.
        if configure.is_resizing() != self.resizing {
            self.resizing = configure.is_resizing();
            if self.resizing {
                self.overlay_subsurface.set_sync();
            } else {
                self.overlay_subsurface.set_desync();
            }
        }

        self.maximized = configure.is_maximized();

        let was_configured = self.configured;
        let old_size = (self.width, self.height);

        if let Some(width) = configure.new_size.0 {
            self.width = width.get();
        }
        if let Some(height) = configure.new_size.1 {
            self.height = height.get();
        }
        let size = Size::new(self.width as f32, self.height as f32);

        *self.shared.size.lock().expect("size mutex poisoned") =
            (self.width, self.height);

        self.apply_layout();
        self.configured = true;

        // Window lifecycle events for the UI: first configure is the open; later
        // ones may report a new size; activation maps to focus.
        if !was_configured {
            self.send_window_event(window::Event::Opened {
                position: None,
                size,
            });
        } else if (self.width, self.height) != old_size {
            self.send_window_event(window::Event::Resized(size));
        }

        if configure.is_activated() != self.focused {
            self.focused = configure.is_activated();
            self.send_window_event(
                if self.focused {
                    window::Event::Focused
                } else {
                    window::Event::Unfocused
                },
            );
        }

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
            Capability::Pointer if self.themed_pointer.is_none() => {
                match self.seat_state.get_pointer_with_theme(
                    qh,
                    &seat,
                    self.shm.wl_shm(),
                    self.cursor_surface.clone(),
                    ThemeSpec::default(),
                ) {
                    Ok(pointer) => self.themed_pointer = Some(pointer),
                    Err(err) => tracing::warn!("failed to get themed pointer: {err}"),
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
                // IME pairs with keyboard focus; create the text input here.
                self.ensure_text_input(&seat, qh);
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
                if let Some(pointer) = self.themed_pointer.take() {
                    pointer.pointer().release();
                }
                self.pointer_on_surface = false;
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
                    // The compositor resets the cursor on enter, so force the
                    // next sync to re-apply it.
                    self.pointer_on_surface = true;
                    self.applied_cursor = None;
                    self.send(Command::Cursor(Some(position)));
                    self.send(Command::Event(Event::Mouse(mouse::Event::CursorEntered)));
                    self.send(Command::Event(Event::Mouse(mouse::Event::CursorMoved {
                        position,
                    })));
                },
                PointerEventKind::Leave { .. } => {
                    self.pointer_on_surface = false;
                    self.send(Command::Cursor(None));
                    self.send(Command::Event(Event::Mouse(mouse::Event::CursorLeft)));
                },
                PointerEventKind::Motion { .. } => {
                    self.send(Command::Cursor(Some(position)));
                    self.send(Command::Event(Event::Mouse(mouse::Event::CursorMoved {
                        position,
                    })));
                },
                PointerEventKind::Press { button, .. } => {
                    self.send(Command::Event(Event::Mouse(
                        mouse::Event::ButtonPressed(input::mouse_button(*button)),
                    )));
                },
                PointerEventKind::Release { button, .. } => {
                    self.send(Command::Event(Event::Mouse(
                        mouse::Event::ButtonReleased(input::mouse_button(*button)),
                    )));
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
                    self.send(Command::Event(Event::Mouse(
                        mouse::Event::WheelScrolled { delta },
                    )));
                },
            }
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
        self.send(Command::Event(Event::Keyboard(input::key_event(
            event.keysym,
            event.utf8,
            self.modifiers,
            true,
            false,
        ))));
    }

    fn repeat_key(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _keyboard: &wl_keyboard::WlKeyboard,
        _serial: u32,
        event: KeyEvent,
    ) {
        self.send(Command::Event(Event::Keyboard(input::key_event(
            event.keysym,
            event.utf8,
            self.modifiers,
            true,
            true,
        ))));
    }

    fn release_key(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _keyboard: &wl_keyboard::WlKeyboard,
        _serial: u32,
        event: KeyEvent,
    ) {
        self.send(Command::Event(Event::Keyboard(input::key_event(
            event.keysym,
            event.utf8,
            self.modifiers,
            false,
            false,
        ))));
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
        self.send(Command::Event(Event::Keyboard(
            keyboard::Event::ModifiersChanged(self.modifiers),
        )));
    }
}

impl ShmHandler for State {
    fn shm_state(&mut self) -> &mut Shm {
        &mut self.shm
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
