use std::sync::atomic::Ordering;
use std::sync::{Arc, mpsc as sync_chan};
use std::time::Duration;

use iced::Program;
use iced_futures::futures::channel::{mpsc, oneshot};
use iced_futures::{Executor as _, Runtime, subscription};
use iced_graphics::compositor::{self, Compositor};
use iced_graphics::{Settings, Shell, Viewport};
use iced_program::Instance;
use iced_runtime::clipboard::Action as ClipboardAction;
use iced_runtime::core::clipboard::{Clipboard, Kind};
use iced_runtime::core::time::Instant;
use iced_runtime::core::{
    Color,
    Event,
    Point,
    Renderer,
    Size,
    input_method,
    mouse,
    renderer,
    theme,
    widget,
    window,
};
use iced_runtime::image::Action as ImageAction;
use iced_runtime::user_interface::{self, Cache, UserInterface};
use iced_runtime::window::Action as WindowAction;
use iced_runtime::{Action, task};
use raw_window_handle::HasDisplayHandle;

use crate::error::Error;
use crate::handle::Shared;

/// How long the render thread waits for a command before re-checking for async
/// runtime output and pending animation frames.
const REDRAW_INTERVAL: Duration = Duration::from_millis(16);

/// A message from the Wayland event loop to the overlay render thread.
pub(crate) enum Command {
    /// An input event to feed to the Iced UI.
    Event(Event),
    /// The cursor moved to a logical position, or left the surface (`None`).
    Cursor(Option<Point>),
    /// The window's logical size and/or integer scale changed.
    Resize { width: u32, height: u32, scale: f64 },
}

/// A window-control request from the overlay UI back to the event-loop thread,
/// which owns the toplevel. The overlay is the window's chrome, so its
/// `window::Action`s drive the real toplevel.
pub(crate) enum WindowRequest {
    /// Set the window title.
    SetTitle(String),
    /// Minimize the window.
    Minimize,
    /// Maximize (`true`) or unmaximize (`false`) the window.
    SetMaximized(bool),
    /// Toggle the maximized state.
    ToggleMaximized,
    /// Enter (`true`) or leave (`false`) fullscreen.
    SetFullscreen(bool),
    /// Begin an interactive move driven by the compositor.
    Drag,
    /// Begin an interactive resize from the given edge/corner.
    DragResize(window::Direction),
    /// Set the minimum size hint (physical-independent logical pixels).
    SetMinSize(Option<(u32, u32)>),
    /// Set the maximum size hint.
    SetMaxSize(Option<(u32, u32)>),
    /// Show the compositor's window menu at the pointer.
    ShowSystemMenu,
    /// Toggle server-side decorations.
    ToggleDecorations,
    /// Report whether the window is maximized.
    GetMaximized(oneshot::Sender<bool>),
    /// Report the current window mode (windowed/fullscreen).
    GetMode(oneshot::Sender<window::Mode>),
    /// Report the size of the monitor the window is on, if known.
    GetMonitorSize(oneshot::Sender<Option<Size>>),
    /// Report a raw identifier for the window (the `wl_surface` protocol id).
    GetRawId(oneshot::Sender<u64>),
}

/// Build the overlay on this (render) thread and drive it until close.
///
/// `target` provides the raw window/display handles of the overlay surface the
/// platform backend created; the renderer is built against it here. The Iced
/// program and wgpu renderer are not `Send`, so they are built on this thread
/// rather than moved across threads; only the `build` closure and the (`Send`)
/// `target` cross the boundary. The build result is reported through `ready` so
/// the spawning thread can fail fast. Running the renderer on its own thread
/// lets it block on surface presentation without stalling the platform event
/// loop (which must keep dispatching for the caller to map the main surface).
#[allow(clippy::too_many_arguments)]
pub(crate) fn run<P, F, W>(
    build: F,
    target: W,
    size: (u32, u32),
    scale: f64,
    commands: sync_chan::Receiver<Command>,
    window_requests: sync_chan::Sender<WindowRequest>,
    shared: Arc<Shared>,
    ready: sync_chan::Sender<Result<(), Error>>,
) where
    F: FnOnce() -> P,
    P: Program,
    W: compositor::Window + Clone,
{
    let overlay = match IcedOverlay::new(
        build(),
        target,
        size.0,
        size.1,
        scale,
        window_requests,
    ) {
        Ok(overlay) => {
            let _ = ready.send(Ok(()));
            overlay
        },
        Err(error) => {
            let _ = ready.send(Err(error));
            return;
        },
    };

    render_loop(overlay, commands, shared);
}

/// Drive the overlay: apply commands, pump the runtime, and redraw as needed.
fn render_loop<P: Program>(
    mut overlay: IcedOverlay<P>,
    commands: sync_chan::Receiver<Command>,
    shared: Arc<Shared>,
) {
    let mut published_cursor = overlay.mouse_interaction();
    let mut published_ime = overlay.input_method().clone();

    while !shared.close_requested.load(Ordering::Acquire) {
        let mut dirty = match commands.recv_timeout(REDRAW_INTERVAL) {
            Ok(command) => apply_command(&mut overlay, command),
            Err(sync_chan::RecvTimeoutError::Timeout) => false,
            Err(sync_chan::RecvTimeoutError::Disconnected) => break,
        };
        while let Ok(command) = commands.try_recv() {
            dirty |= apply_command(&mut overlay, command);
        }

        dirty |= overlay.pump();

        if overlay.should_exit() {
            shared.close_requested.store(true, Ordering::Release);
            break;
        }

        if dirty || overlay.wants_redraw() {
            overlay.draw();

            // Publish the cursor the UI wants so the event-loop thread (which
            // owns the pointer) can apply it.
            let interaction = overlay.mouse_interaction();
            if interaction != published_cursor {
                published_cursor = interaction;
                *shared.cursor.lock().expect("cursor mutex poisoned") = interaction;
            }

            // Likewise publish the input-method state for the text input.
            if *overlay.input_method() != published_ime {
                published_ime = overlay.input_method().clone();
                *shared.ime.lock().expect("ime mutex poisoned") = published_ime.clone();
            }
        }
    }
}

/// Apply a single [`Command`], returning whether it requires a redraw.
fn apply_command<P: Program>(overlay: &mut IcedOverlay<P>, command: Command) -> bool {
    match command {
        Command::Event(event) => {
            overlay.queue_event(event);
            true
        },
        Command::Cursor(position) => {
            overlay.set_cursor(position);
            false
        },
        Command::Resize {
            width,
            height,
            scale,
        } => {
            overlay.resize(width, height, scale);
            true
        },
    }
}

/// The compositor type associated with a program's renderer.
type CompositorOf<P> = <<P as Program>::Renderer as compositor::Default>::Compositor;

/// The surface type produced by that compositor.
type SurfaceOf<P> = <CompositorOf<P> as Compositor>::Surface;

/// Upper bound on a surface dimension, clamped to stay within GPU texture limits.
///
/// iced creates the wgpu device with wgpu's default limits, whose
/// `max_texture_dimension_2d` is 8192 regardless of the hardware maximum.
const MAX_SURFACE_DIMENSION: u32 = 8192;

/// Convert a logical size at a given scale to a physical pixel size.
///
/// Clamped to `[1, MAX_SURFACE_DIMENSION]` so an oversized window cannot exceed
/// the renderer's maximum texture dimension.
fn physical_size(logical_width: u32, logical_height: u32, scale: f64) -> (u32, u32) {
    let to_physical = |logical: u32| {
        ((logical as f64) * scale)
            .round()
            .clamp(1.0, MAX_SURFACE_DIMENSION as f64) as u32
    };
    (to_physical(logical_width), to_physical(logical_height))
}

/// Convert an optional logical [`Size`] to integer dimensions for size hints.
fn to_dimensions(size: Option<Size>) -> Option<(u32, u32)> {
    size.map(|size| (size.width as u32, size.height as u32))
}

/// The runtime action type produced by a program's messages.
type ActionOf<P> = Action<<P as Program>::Message>;

/// The Iced runtime specialised for a program, sending actions over an mpsc channel.
type RuntimeOf<P> =
    Runtime<<P as Program>::Executor, mpsc::UnboundedSender<ActionOf<P>>, ActionOf<P>>;

/// Bridges iced's [`clipboard::Clipboard`] to the platform clipboard.
///
/// `window_clipboard` selects the backend per platform (smithay-clipboard on
/// Wayland). Connecting can fail (e.g. no seat / headless); a clipboard that
/// failed to connect reads empty and silently drops writes.
struct OverlayClipboard {
    inner: Option<window_clipboard::Clipboard>,
}

impl OverlayClipboard {
    /// Connect to the platform clipboard via the surface's display handle.
    fn connect<W: HasDisplayHandle>(target: &W) -> Self {
        // SAFETY: the display handle stays valid for the clipboard's lifetime —
        // the overlay that owns this clipboard is dropped before the Wayland
        // connection is torn down on the event-loop thread.
        let inner = unsafe { window_clipboard::Clipboard::connect(target) }.ok();
        Self { inner }
    }
}

impl Clipboard for OverlayClipboard {
    fn read(&self, kind: Kind) -> Option<String> {
        let clipboard = self.inner.as_ref()?;
        match kind {
            Kind::Standard => clipboard.read().ok(),
            Kind::Primary => clipboard.read_primary().and_then(Result::ok),
        }
    }

    fn write(&mut self, kind: Kind, contents: String) {
        let Some(clipboard) = self.inner.as_mut() else {
            return;
        };
        let _ = match kind {
            Kind::Standard => clipboard.write(contents),
            Kind::Primary => clipboard.write_primary(contents).unwrap_or(Ok(())),
        };
    }
}

/// An Iced program rendered into a platform surface via its wgpu compositor.
pub(crate) struct IcedOverlay<P: Program> {
    instance: Instance<P>,
    window_id: window::Id,
    compositor: CompositorOf<P>,
    renderer: P::Renderer,
    surface: SurfaceOf<P>,
    clipboard: OverlayClipboard,
    default_theme: P::Theme,
    cache: Option<Cache>,
    viewport: Viewport,
    // Physical (buffer) pixel size, the logical size, and the two scale factors:
    // `compositor_scale` is the integer HiDPI scale (drives the buffer); the
    // viewport's scale is `compositor_scale * program_scale`, where the program
    // scale is the app's own zoom from `Instance::scale_factor`.
    width: u32,
    height: u32,
    logical_width: u32,
    logical_height: u32,
    compositor_scale: f64,
    program_scale: f64,
    last_title: String,
    events: Vec<Event>,
    cursor: mouse::Cursor,
    mouse_interaction: mouse::Interaction,
    input_method: input_method::InputMethod,
    runtime: RuntimeOf<P>,
    receiver: mpsc::UnboundedReceiver<ActionOf<P>>,
    window_requests: sync_chan::Sender<WindowRequest>,
    redraw_request: window::RedrawRequest,
    exit: bool,
}

impl<P: Program> IcedOverlay<P> {
    /// Build the renderer/compositor for `program` on the given overlay surface.
    pub(crate) fn new<W>(
        program: P,
        target: W,
        width: u32,
        height: u32,
        scale: f64,
        window_requests: sync_chan::Sender<WindowRequest>,
    ) -> Result<Self, Error>
    where
        W: compositor::Window + Clone,
    {
        let clipboard = OverlayClipboard::connect(&target);

        let mut compositor = pollster::block_on(CompositorOf::<P>::new(
            Settings::default(),
            target.clone(),
            target.clone(),
            Shell::headless(),
        ))
        .map_err(|err| Error::RendererInit {
            message: err.to_string(),
        })?;

        let (physical_width, physical_height) = physical_size(width, height, scale);
        let renderer = compositor.create_renderer();
        let surface = compositor.create_surface(target, physical_width, physical_height);

        let executor =
            P::Executor::new().map_err(|source| Error::Executor { source })?;
        let (sender, receiver) = mpsc::unbounded();
        let mut runtime = RuntimeOf::<P>::new(executor, sender);

        let (instance, boot_task) = Instance::new(program);
        let window_id = window::Id::unique();
        let default_theme = <P::Theme as theme::Base>::default(theme::Mode::default());
        let program_scale = instance.scale_factor(window_id) as f64;
        let viewport = Viewport::with_physical_size(
            Size::new(physical_width, physical_height),
            (scale * program_scale) as f32,
        );

        if let Some(stream) = task::into_stream(boot_task) {
            runtime.run(stream);
        }

        let mut overlay = Self {
            instance,
            window_id,
            compositor,
            renderer,
            surface,
            clipboard,
            window_requests,
            default_theme,
            cache: Some(Cache::default()),
            viewport,
            width: physical_width,
            height: physical_height,
            logical_width: width,
            logical_height: height,
            compositor_scale: scale,
            program_scale,
            last_title: String::new(),
            events: Vec::new(),
            cursor: mouse::Cursor::Unavailable,
            mouse_interaction: mouse::Interaction::None,
            input_method: input_method::InputMethod::Disabled,
            runtime,
            receiver,
            redraw_request: window::RedrawRequest::Wait,
            exit: false,
        };

        overlay.sync_subscriptions();
        overlay.sync_title();
        Ok(overlay)
    }

    /// Forward the program's window title to the toplevel if it changed.
    fn sync_title(&mut self) {
        let title = self.instance.title(self.window_id);
        if title != self.last_title {
            self.last_title.clone_from(&title);
            let _ = self.window_requests.send(WindowRequest::SetTitle(title));
        }
    }

    /// Apply a message to the program state, running any resulting [`Task`].
    ///
    /// [`Task`]: iced_runtime::Task
    fn apply_message(&mut self, message: P::Message) {
        let task = self.instance.update(message);
        if let Some(stream) = task::into_stream(task) {
            self.runtime.run(stream);
        }
    }

    /// Recompute the program's [`Subscription`] and reconcile it in the runtime.
    ///
    /// [`Subscription`]: iced_futures::Subscription
    fn sync_subscriptions(&mut self) {
        let subscription = self.runtime.enter(|| self.instance.subscription());
        self.runtime
            .track(subscription::into_recipes(subscription.map(Action::Output)));
    }
}

impl<P: Program> IcedOverlay<P> {
    fn resize(&mut self, logical_width: u32, logical_height: u32, scale: f64) {
        let (width, height) = physical_size(logical_width, logical_height, scale);
        if self.width == width && self.height == height && self.compositor_scale == scale
        {
            return;
        }

        self.logical_width = logical_width;
        self.logical_height = logical_height;
        self.compositor_scale = scale;
        self.width = width;
        self.height = height;
        self.update_viewport();
        self.compositor
            .configure_surface(&mut self.surface, width, height);
    }

    /// Rebuild the viewport from the current physical size and effective scale.
    ///
    /// The buffer (physical) size is unaffected by the program scale, so callers
    /// that only change the program scale need not reconfigure the wgpu surface.
    fn update_viewport(&mut self) {
        let scale = (self.compositor_scale * self.program_scale) as f32;
        self.viewport =
            Viewport::with_physical_size(Size::new(self.width, self.height), scale);
    }

    fn queue_event(&mut self, event: Event) {
        self.events.push(event);
    }

    fn set_cursor(&mut self, position: Option<Point>) {
        self.cursor = match position {
            Some(position) => mouse::Cursor::Available(position),
            None => mouse::Cursor::Unavailable,
        };
    }

    fn pump(&mut self) -> bool {
        let mut applied = false;
        let mut redraw = false;

        while let Ok(Some(action)) = self.receiver.try_next() {
            match action {
                Action::Output(message) => {
                    self.apply_message(message);
                    applied = true;
                },
                Action::LoadFont { bytes, channel } => {
                    self.compositor.load_font(bytes);
                    let _ = channel.send(Ok(()));
                    redraw = true;
                },
                Action::Widget(operation) => {
                    self.apply_operation(operation);
                    redraw = true;
                },
                Action::Clipboard(ClipboardAction::Read { target, channel }) => {
                    let _ = channel.send(self.clipboard.read(target));
                },
                Action::Clipboard(ClipboardAction::Write { target, contents }) => {
                    self.clipboard.write(target, contents);
                },
                Action::Window(action) => self.apply_window_action(action),
                Action::Image(ImageAction::Allocate(handle, sender)) => {
                    self.renderer.allocate_image(&handle, move |allocation| {
                        let _ = sender.send(allocation);
                    });
                    redraw = true;
                },
                Action::Exit => self.exit = true,
                // System actions (theme/info) are not supported; ignore them.
                _ => {},
            }
        }

        if applied {
            self.sync_subscriptions();
        }

        applied || redraw
    }

    /// Handle a window [`Action`] from the program.
    ///
    /// Queries the overlay can answer itself (size, scale, id) are replied to
    /// directly; window-control actions are forwarded to the event-loop thread,
    /// which owns the toplevel; `Close` ends the loop like [`Action::Exit`].
    /// Geometry actions that Wayland leaves to the compositor are ignored.
    ///
    /// [`Action`]: WindowAction
    fn apply_window_action(&mut self, action: WindowAction) {
        match action {
            WindowAction::Close(_) => self.exit = true,
            WindowAction::GetSize(_, channel) => {
                let _ = channel.send(self.viewport.logical_size());
            },
            WindowAction::GetScaleFactor(_, channel) => {
                let _ =
                    channel.send((self.compositor_scale * self.program_scale) as f32);
            },
            WindowAction::GetLatest(channel) | WindowAction::GetOldest(channel) => {
                let _ = channel.send(Some(self.window_id));
            },
            WindowAction::Minimize(_, true)
            | WindowAction::SetMode(_, window::Mode::Hidden) => {
                self.request_window(WindowRequest::Minimize);
            },
            WindowAction::Maximize(_, maximized) => {
                self.request_window(WindowRequest::SetMaximized(maximized));
            },
            WindowAction::ToggleMaximize(_) => {
                self.request_window(WindowRequest::ToggleMaximized);
            },
            WindowAction::SetMode(_, window::Mode::Fullscreen) => {
                self.request_window(WindowRequest::SetFullscreen(true));
            },
            WindowAction::SetMode(_, window::Mode::Windowed) => {
                self.request_window(WindowRequest::SetFullscreen(false));
            },
            WindowAction::Drag(_) => self.request_window(WindowRequest::Drag),
            WindowAction::DragResize(_, direction) => {
                self.request_window(WindowRequest::DragResize(direction));
            },
            WindowAction::GetPosition(_, channel) => {
                // Wayland does not expose a global window position.
                let _ = channel.send(None);
            },
            WindowAction::Screenshot(_, channel) => {
                let bytes = self.compositor.screenshot(
                    &mut self.renderer,
                    &self.viewport,
                    Color::TRANSPARENT,
                );
                let _ = channel.send(window::Screenshot::new(
                    bytes,
                    Size::new(self.width, self.height),
                    self.viewport.scale_factor(),
                ));
            },
            WindowAction::RedrawAll | WindowAction::RelayoutAll => {
                self.redraw_request = window::RedrawRequest::NextFrame;
            },
            WindowAction::GetMinimized(_, channel) => {
                // Wayland does not tell clients whether they are minimized.
                let _ = channel.send(None);
            },
            WindowAction::SetMinSize(_, size) => {
                self.request_window(WindowRequest::SetMinSize(to_dimensions(size)));
            },
            WindowAction::SetMaxSize(_, size) => {
                self.request_window(WindowRequest::SetMaxSize(to_dimensions(size)));
            },
            WindowAction::ShowSystemMenu(_) => {
                self.request_window(WindowRequest::ShowSystemMenu);
            },
            WindowAction::ToggleDecorations(_) => {
                self.request_window(WindowRequest::ToggleDecorations);
            },
            // State queries the event-loop thread (which owns the toplevel)
            // answers; the reply channel travels with the request.
            WindowAction::GetMaximized(_, channel) => {
                self.request_window(WindowRequest::GetMaximized(channel));
            },
            WindowAction::GetMode(_, channel) => {
                self.request_window(WindowRequest::GetMode(channel));
            },
            WindowAction::GetMonitorSize(_, channel) => {
                self.request_window(WindowRequest::GetMonitorSize(channel));
            },
            WindowAction::GetRawId(_, channel) => {
                self.request_window(WindowRequest::GetRawId(channel));
            },
            // Geometry/position/icon/level/etc. are compositor-owned on Wayland
            // (or have no equivalent); ignore them.
            _ => {},
        }
    }

    /// Forward a window-control request to the event-loop thread.
    fn request_window(&self, request: WindowRequest) {
        let _ = self.window_requests.send(request);
    }

    /// Apply a widget [`Operation`] (focus, scroll-to, queries, ...) to the UI.
    ///
    /// Operations run against a freshly built interface so they see current
    /// widget state, and the resulting cache is kept for the next draw so focus
    /// and similar state persist. Chained operations are followed to completion.
    ///
    /// [`Operation`]: widget::Operation
    fn apply_operation(&mut self, operation: Box<dyn widget::Operation>) {
        let bounds = self.viewport.logical_size();
        let mut cache = self.cache.take().unwrap_or_default();
        let mut current = Some(operation);

        while let Some(mut operation) = current.take() {
            let mut ui = UserInterface::build(
                self.instance.view(self.window_id),
                bounds,
                cache,
                &mut self.renderer,
            );
            ui.operate(&self.renderer, operation.as_mut());
            cache = ui.into_cache();

            current = match operation.finish() {
                widget::operation::Outcome::Chain(next) => Some(next),
                _ => None,
            };
        }

        self.cache = Some(cache);
    }

    /// The cursor the UI currently wants shown, as of the last [`Self::draw`].
    fn mouse_interaction(&self) -> mouse::Interaction {
        self.mouse_interaction
    }

    /// The input-method state the UI wants, as of the last [`Self::draw`].
    fn input_method(&self) -> &input_method::InputMethod {
        &self.input_method
    }

    fn wants_redraw(&self) -> bool {
        match self.redraw_request {
            window::RedrawRequest::NextFrame => true,
            window::RedrawRequest::At(instant) => Instant::now() >= instant,
            window::RedrawRequest::Wait => false,
        }
    }

    fn should_exit(&self) -> bool {
        self.exit
    }

    fn draw(&mut self) {
        // The program's own scale factor can change between frames; fold it into
        // the viewport (buffer size is unaffected, so no surface reconfigure).
        let program_scale = self.instance.scale_factor(self.window_id) as f64;
        if program_scale != self.program_scale {
            self.program_scale = program_scale;
            self.update_viewport();
        }

        let bounds = self.viewport.logical_size();
        let mut cache = self.cache.take().unwrap_or_default();

        // A redraw-request event is delivered every frame (matching iced_winit)
        // so time-based animations and the `window::frames()` subscription keep
        // advancing without input; queued input events follow it. Running the
        // update pass also refreshes the animation redraw request.
        let redraw_event = Event::Window(window::Event::RedrawRequested(Instant::now()));
        let mut events = vec![redraw_event];
        events.append(&mut self.events);

        let mut messages = Vec::new();
        let statuses = {
            let mut ui = UserInterface::build(
                self.instance.view(self.window_id),
                bounds,
                cache,
                &mut self.renderer,
            );
            let (state, statuses) = ui.update(
                &events,
                self.cursor,
                &mut self.renderer,
                &mut self.clipboard,
                &mut messages,
            );
            match state {
                user_interface::State::Updated {
                    redraw_request,
                    mouse_interaction,
                    input_method,
                    ..
                } => {
                    self.redraw_request = redraw_request;
                    self.mouse_interaction = mouse_interaction;
                    self.input_method = input_method;
                },
                user_interface::State::Outdated => {
                    self.redraw_request = window::RedrawRequest::NextFrame;
                },
            }
            cache = ui.into_cache();
            statuses
        };

        // Forward every processed event (the redraw tick plus inputs) to the
        // runtime so event-listening and redraw-driven subscriptions fire.
        for (event, status) in events.into_iter().zip(statuses) {
            self.runtime.broadcast(subscription::Event::Interaction {
                window: self.window_id,
                event,
                status,
            });
        }

        let applied = !messages.is_empty();
        for message in messages {
            self.apply_message(message);
        }
        if applied {
            self.sync_subscriptions();
            // The title may depend on state the messages just changed.
            self.sync_title();
        }

        let theme = self.instance.theme(self.window_id);
        let theme = theme.as_ref().unwrap_or(&self.default_theme);
        let text_color = self.instance.style(theme).text_color;

        let mut ui = UserInterface::build(
            self.instance.view(self.window_id),
            bounds,
            cache,
            &mut self.renderer,
        );
        ui.draw(
            &mut self.renderer,
            theme,
            &renderer::Style { text_color },
            self.cursor,
        );

        self.cache = Some(ui.into_cache());

        match self.compositor.present(
            &mut self.renderer,
            &mut self.surface,
            &self.viewport,
            Color::TRANSPARENT,
            || {},
        ) {
            Ok(()) => {},
            Err(compositor::SurfaceError::Outdated | compositor::SurfaceError::Lost) => {
                self.compositor.configure_surface(
                    &mut self.surface,
                    self.width.max(1),
                    self.height.max(1),
                );
                // The frame we tried to present was dropped; ask the loop to
                // repaint so the overlay does not stay blank until the next
                // input or animation frame happens to arrive.
                self.redraw_request = window::RedrawRequest::NextFrame;
            },
            Err(err) => {
                tracing::warn!("overlay present failed: {err}");
            },
        }
    }
}
